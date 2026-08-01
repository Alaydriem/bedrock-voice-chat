use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use tokio::task::JoinSet;

use super::{HttpsProbe, NegotiationProbe};
use crate::structs::reachability::{
    AddressFamily, AddressFamilyPreference, EndpointReachability, ReachabilityOutcome,
    ReachabilityRequest, ServerReachability,
};

pub struct ReachabilityProbe {
    cache: Cache<String, ServerReachability>,
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
        if let Some(cached) = self.cache.get(&request.host).await {
            return cached;
        }

        let report = Self::measure(request).await;
        self.cache
            .insert(request.host.clone(), report.clone())
            .await;
        report
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

    async fn measure(request: &ReachabilityRequest) -> ServerReachability {
        let mut quic_tasks = JoinSet::new();
        for addr in &request.addrs {
            for port in &request.quic_ports {
                let dest = SocketAddr::new(*addr, *port);
                let server_name = request.host.clone();
                quic_tasks.spawn(async move { Self::measure_quic(dest, server_name).await });
            }
        }

        let mut https_tasks = JoinSet::new();
        for addr in &request.addrs {
            let url = request.https_url.clone();
            let dest = SocketAddr::new(*addr, request.https_port);
            let family = AddressFamily::of(addr);
            https_tasks.spawn(async move {
                let outcome = HttpsProbe::probe(&url, family).await;
                EndpointReachability::new(dest, outcome, None)
            });
        }

        let mut quic = Vec::new();
        while let Some(joined) = quic_tasks.join_next().await {
            if let Ok(endpoint) = joined {
                quic.push(endpoint);
            }
        }

        let mut https = Vec::new();
        while let Some(joined) = https_tasks.join_next().await {
            if let Ok(endpoint) = joined {
                https.push(endpoint);
            }
        }

        ServerReachability::new(request.host.clone(), quic, https)
    }

    // Escalates only when it has to. The negotiation probe costs one round trip and
    // no certificates; the handshake probe costs a TLS exchange but carries SNI,
    // which is the only way to reach a server behind an SNI-routing proxy.
    async fn measure_quic(dest: SocketAddr, server_name: String) -> EndpointReachability {
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
