use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;

use super::{HttpsProbe, MeasuredLeg, NegotiationProbe, WsVoiceProbe};
use crate::net::NetTimeouts;
use crate::structs::reachability::{
    AddressFamily, AddressFamilyPreference, EndpointReachability, ReachabilityOutcome,
    ReachabilityRequest, ReachabilityVerdict, ServerReachability,
};

pub struct ReachabilityProbe {
    // The measured port set travels with the report. Keyed on host alone a preflight
    // over one port would answer a later connect that dials more, and the verdict
    // would be asserting something about an endpoint nobody probed.
    //
    // The flag beside it is whether the fallback transport was measured, for the same
    // reason: a report taken before the server's capability was known carries no
    // WebSocket leg, and handing it to a caller that asked for one would report no
    // fallback path on a server that has one.
    cache: Cache<String, (Vec<u16>, bool, ServerReachability)>,
}

impl ReachabilityProbe {
    // Long enough that connecting twice in a row does not re-probe, short enough
    // that a host which changed networks re-measures on its own. A failed connect
    // invalidates the entry, so this is the ceiling on staleness rather than the
    // usual path to a refresh.
    const TTL: Duration = Duration::from_secs(300);
    const CAPACITY: u64 = 64;

    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Self::TTL)
                .max_capacity(Self::CAPACITY)
                .build(),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub async fn evaluate(&self, request: &ReachabilityRequest) -> ServerReachability {
        if let Some(cached) = self.cached(request).await {
            return cached;
        }

        let report = Self::measure(request, None).await;
        self.store(request, &report).await;
        report
    }

    /// The complete measurement, or the first one that proved voice can get through.
    ///
    /// For a screen that only asks "can voice reach this address", where waiting out the QUIC
    /// budget costs seconds for an answer the WebSocket probe already gave in milliseconds.
    ///
    /// **The returned report may be incomplete.** An early answer means one leg finished and
    /// the others did not, so its verdict says a path exists and nothing more — in particular
    /// `VoiceFallback` here means QUIC had not answered *yet*, not that it is blocked. It must
    /// never reach `CandidatePlan::build`, which orders the walk by per-endpoint latency the
    /// early report does not have.
    ///
    /// The measurement itself always runs to completion in the background and caches the whole
    /// report, so the connect that follows reads a complete one rather than paying for the
    /// walk a second time.
    pub async fn evaluate_any_voice_path(
        self: &Arc<Self>,
        request: &ReachabilityRequest,
    ) -> ServerReachability {
        if let Some(cached) = self.cached(request).await {
            return cached;
        }

        // Two capacity: the first positive interim and the final report. A sender that cannot
        // queue would make the measurement wait on a receiver that has already left.
        let (tx, mut rx) = mpsc::channel::<ServerReachability>(2);
        let probe = self.clone();
        let owned = request.clone();

        tokio::spawn(async move {
            let report = Self::measure(&owned, Some(tx.clone())).await;
            probe.store(&owned, &report).await;
            let _ = tx.send(report).await;
        });

        match rx.recv().await {
            Some(report) => report,
            // The measurement task died before it said anything. Measuring here rather than
            // inventing a verdict: an "unreachable" this never established would send somebody
            // to ask their server operator about a fault on this device.
            None => Self::measure(request, None).await,
        }
    }

    // A cached report answers only if it measured every port being asked about, and the
    // fallback leg if one was asked for. A wider one answers a narrower question; a
    // narrower one never answers a wider one.
    async fn cached(&self, request: &ReachabilityRequest) -> Option<ServerReachability> {
        let (measured, measured_ws, cached) = self.cache.get(&request.host).await?;

        let ports_covered = request
            .quic_ports
            .iter()
            .all(|port| measured.contains(port));

        (ports_covered && (measured_ws || !request.voice_websocket)).then_some(cached)
    }

    async fn store(&self, request: &ReachabilityRequest, report: &ServerReachability) {
        self.cache
            .insert(
                request.host.clone(),
                (
                    request.quic_ports.clone(),
                    request.voice_websocket,
                    report.clone(),
                ),
            )
            .await;
    }

    pub async fn preference(&self, request: &ReachabilityRequest) -> AddressFamilyPreference {
        self.evaluate(request).await.preference()
    }

    pub async fn invalidate(&self, host: &str) {
        self.cache.invalidate(host).await;
    }

    pub fn is_cached(&self, host: &str) -> bool {
        self.cache.contains_key(host)
    }

    /// Runs every leg and returns the complete report.
    ///
    /// `first_path` receives one interim report, the moment the legs that have landed prove
    /// voice can get through. Only good news is published: a verdict derived while the QUIC
    /// walk is still running would read "no voice path" about a server that is about to
    /// answer, and correcting that a few seconds later is worse than saying nothing.
    async fn measure(
        request: &ReachabilityRequest,
        first_path: Option<mpsc::Sender<ServerReachability>>,
    ) -> ServerReachability {
        let mut tasks = JoinSet::new();

        // Stops the QUIC legs that are still running once one of them has answered. Only
        // they are cancelled: the fallback's round trip decides which transport the
        // connect dials, and the HTTPS leg is what separates a blocked path from a host
        // that is not there.
        let (settle_tx, settle_rx) = watch::channel(false);
        let settle_tx = Arc::new(settle_tx);

        for addr in &request.addrs {
            for port in &request.quic_ports {
                let dest = SocketAddr::new(*addr, *port);
                let server_name = request.host.clone();
                let settled = settle_rx.clone();
                tasks.spawn(async move {
                    MeasuredLeg::Quic(Self::measure_quic(dest, server_name, settled).await)
                });
            }
        }
        drop(settle_rx);

        for addr in &request.addrs {
            let url = request.https_url.clone();
            let dest = SocketAddr::new(*addr, request.https_port);
            let family = AddressFamily::of(addr);
            tasks.spawn(async move {
                let outcome = HttpsProbe::probe(&url, family).await;
                MeasuredLeg::Https(EndpointReachability::new(dest, outcome, None))
            });
        }

        // Only run against a server that claimed the transport. An unadvertised one would
        // answer the ALPN with a refusal, and a whole TLS handshake spent confirming what
        // `/api/config` already said is time the screen waiting on it pays for.
        if request.voice_websocket {
            for addr in &request.addrs {
                let dest = SocketAddr::new(*addr, request.https_port);
                let server_name = request.host.clone();
                tasks.spawn(async move {
                    let outcome = WsVoiceProbe::probe(dest, &server_name).await;
                    MeasuredLeg::Ws(EndpointReachability::new(dest, outcome, None))
                });
            }
        }

        let mut quic = Vec::new();
        let mut https = Vec::new();
        let mut ws = Vec::new();
        let mut announced = false;

        let mut settling = false;

        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok(MeasuredLeg::Quic(endpoint)) => {
                    // One answer is the whole of what the walk needs. An endpoint still
                    // escalating to a handshake probe can only sort below this one, and
                    // waiting for it to exhaust its budget is what made a dead advertised
                    // port cost seconds on every launch.
                    let answered = endpoint.outcome().answered();
                    quic.push(endpoint);

                    if answered && !settling {
                        settling = true;
                        let settle_tx = settle_tx.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(NetTimeouts::PROBE_SETTLE).await;
                            let _ = settle_tx.send(true);
                        });
                    }
                }
                Ok(MeasuredLeg::Https(endpoint)) => https.push(endpoint),
                Ok(MeasuredLeg::Ws(endpoint)) => ws.push(endpoint),
                Err(_) => continue,
            }

            if announced {
                continue;
            }

            if let Some(sender) = &first_path {
                let interim =
                    ServerReachability::new(request.host.clone(), quic.clone(), https.clone(), ws.clone());
                if Self::carries_voice(interim.verdict()) {
                    announced = true;
                    let _ = sender.send(interim).await;
                }
            }
        }

        ServerReachability::new(request.host.clone(), quic, https, ws)
    }

    // Whether an interim verdict is worth answering with. Both arms mean a transport
    // answered, which is the whole of what an early caller asked.
    fn carries_voice(verdict: ReachabilityVerdict) -> bool {
        matches!(
            verdict,
            ReachabilityVerdict::Ready | ReachabilityVerdict::VoiceFallback
        )
    }

    // Abandoned the moment a sibling endpoint has answered and settled. The result is
    // recorded as silence, which is what it is: within the time this probe was given, this
    // endpoint said nothing. It still sorts last rather than being dropped, and the walk
    // still dials it if the endpoint that did answer fails to carry a session.
    async fn measure_quic(
        dest: SocketAddr,
        server_name: String,
        mut settled: watch::Receiver<bool>,
    ) -> EndpointReachability {
        tokio::select! {
            biased;

            endpoint = Self::probe_quic(dest, server_name) => endpoint,
            _ = Self::settled(&mut settled) => {
                EndpointReachability::new(dest, ReachabilityOutcome::Silent, None)
            }
        }
    }

    // Never resolves once the sender is gone. A dropped sender means the measurement is
    // already finishing, and resolving here would report silence about an endpoint whose
    // own probe was about to answer.
    async fn settled(settled: &mut watch::Receiver<bool>) {
        if settled.wait_for(|done| *done).await.is_err() {
            std::future::pending::<()>().await;
        }
    }

    // Escalates only when it has to. The negotiation probe costs one round trip and
    // no certificates; the handshake probe costs a TLS exchange but carries SNI,
    // which is the only way to reach a server behind an SNI-routing proxy.
    async fn probe_quic(dest: SocketAddr, server_name: String) -> EndpointReachability {
        let negotiation = NegotiationProbe::probe(dest).await;

        if negotiation.answered() || matches!(negotiation, ReachabilityOutcome::NoRoute) {
            return EndpointReachability::new(dest, negotiation, None);
        }

        #[cfg(feature = "quic")]
        {
            let (outcome, certificate) = super::HandshakeProbe::probe(dest, &server_name).await;
            EndpointReachability::new(dest, outcome, certificate)
        }

        #[cfg(not(feature = "quic"))]
        {
            let _ = server_name;
            EndpointReachability::new(dest, negotiation, None)
        }
    }
}

impl Default for ReachabilityProbe {
    fn default() -> Self {
        Self::new()
    }
}
