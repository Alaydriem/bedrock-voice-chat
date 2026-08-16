mod certificate;
mod endpoint;
mod family;
mod outcome;
mod request;
mod verdict;

pub use certificate::ObservedCertificate;
pub use endpoint::EndpointReachability;
pub use family::{AddressFamily, AddressFamilyPreference};
pub use outcome::{AnsweredVia, ReachabilityOutcome};
pub use request::ReachabilityRequest;
pub use verdict::ReachabilityVerdict;

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ServerReachability {
    host: String,
    quic: Vec<EndpointReachability>,
    https: Vec<EndpointReachability>,
    // The WebSocket voice transport, measured on the public TLS port. Empty when the
    // server never advertised the transport, which is not the same as it having been
    // measured and found silent — `verdict` is what distinguishes those.
    ws: Vec<EndpointReachability>,
    preference: AddressFamilyPreference,
    // Derived and stored rather than computed on demand, for the same reason as
    // `preference` and for one more: these are fields, so they cross to the
    // frontend. A method would not, and the UI would end up re-deriving the
    // matrix in TypeScript where it could drift from this one.
    verdict: ReachabilityVerdict,
    best_rtt_micros: Option<u32>,
    // The fallback path's own round trip. Separate from `best_rtt_micros`, which
    // orders QUIC candidates and must stay a QUIC measurement — a TCP figure mixed
    // into it would sort a candidate list by a transport it does not contain.
    fallback_rtt_micros: Option<u32>,
}

impl ServerReachability {
    // Every derived value is computed here rather than supplied, so no caller can
    // publish a report whose verdict disagrees with its own measurements. Only
    // the QUIC endpoints count toward the family preference: HTTPS reports on a
    // different transport, and the family that carries voice is the one the
    // verdict has to be right about.
    pub fn new(
        host: String,
        quic: Vec<EndpointReachability>,
        https: Vec<EndpointReachability>,
        ws: Vec<EndpointReachability>,
    ) -> Self {
        let preference = if quic
            .iter()
            .any(|e| e.family() == AddressFamily::Ipv6 && e.outcome().answered())
        {
            AddressFamilyPreference::PreferIpv6
        } else {
            AddressFamilyPreference::PreferIpv4
        };

        let verdict = Self::derive_verdict(&quic, &https, &ws);
        let best_rtt_micros = Self::lowest_answered_rtt(&quic);
        let fallback_rtt_micros = Self::lowest_answered_rtt(&ws);

        Self {
            host,
            quic,
            https,
            ws,
            preference,
            verdict,
            best_rtt_micros,
            fallback_rtt_micros,
        }
    }

    // Ordering is the whole content of this function. A QUIC answer settles it.
    // Then the fallback transport, because an answer there means voice connects —
    // which outranks anything the remaining arms could say, including a local stack
    // that found no UDP route. Only after both voice transports have failed does
    // HTTPS separate a blocked path from a host that is simply not there.
    fn derive_verdict(
        quic: &[EndpointReachability],
        https: &[EndpointReachability],
        ws: &[EndpointReachability],
    ) -> ReachabilityVerdict {
        if quic.iter().any(|e| e.outcome().answered()) {
            return ReachabilityVerdict::Ready;
        }

        if ws.iter().any(|e| e.outcome().answered()) {
            return ReachabilityVerdict::VoiceFallback;
        }

        let all_unrouted = !quic.is_empty()
            && quic
                .iter()
                .all(|e| matches!(e.outcome(), ReachabilityOutcome::NoRoute));
        if all_unrouted {
            return ReachabilityVerdict::NoRoute;
        }

        if https.iter().any(|e| e.outcome().answered()) {
            return ReachabilityVerdict::VoiceBlocked;
        }

        ReachabilityVerdict::Unreachable
    }

    fn lowest_answered_rtt(quic: &[EndpointReachability]) -> Option<u32> {
        quic.iter()
            .filter(|e| e.outcome().answered())
            .filter_map(|e| e.outcome().rtt_micros())
            .min()
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn quic(&self) -> &[EndpointReachability] {
        &self.quic
    }

    pub fn https(&self) -> &[EndpointReachability] {
        &self.https
    }

    pub fn ws(&self) -> &[EndpointReachability] {
        &self.ws
    }

    pub fn preference(&self) -> AddressFamilyPreference {
        self.preference
    }

    pub fn rtt_for(&self, ip: &IpAddr, port: u16) -> Option<u32> {
        let wanted = SocketAddr::new(*ip, port).to_string();
        self.quic
            .iter()
            .find(|e| e.addr() == wanted)
            .and_then(|e| e.outcome().rtt_micros())
    }

    // The lowest-RTT QUIC endpoint that actually answered. Derived here rather
    // than in the UI so a caption cannot disagree with the ordering
    // CandidatePlan::build will use. HTTPS is excluded: it reports on a
    // different transport from the one that carries voice.
    pub fn best_quic(&self) -> Option<&EndpointReachability> {
        self.quic
            .iter()
            .filter(|e| e.outcome().answered())
            .min_by_key(|e| e.outcome().rtt_micros().unwrap_or(u32::MAX))
    }

    // Whether QUIC has a path. A single answering endpoint is enough: the candidate
    // plan will find it.
    pub fn any_quic_answered(&self) -> bool {
        self.verdict == ReachabilityVerdict::Ready
    }

    // Whether the WebSocket voice transport answered, which is the only evidence that
    // a UDP-blocked client has anywhere to go. A connect that reads this as true when
    // it is not spends a TLS handshake to be refused; reading it as false when it is
    // true strands a player who could have been talking.
    pub fn voice_fallback_answered(&self) -> bool {
        self.verdict == ReachabilityVerdict::VoiceFallback
    }

    // Whether either transport can carry voice to this server from this network.
    pub fn any_voice_path(&self) -> bool {
        matches!(
            self.verdict,
            ReachabilityVerdict::Ready | ReachabilityVerdict::VoiceFallback
        )
    }

    pub fn best_rtt_micros(&self) -> Option<u32> {
        self.best_rtt_micros
    }

    pub fn fallback_rtt_micros(&self) -> Option<u32> {
        self.fallback_rtt_micros
    }

    pub fn verdict(&self) -> ReachabilityVerdict {
        self.verdict
    }
}
