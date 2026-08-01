use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AnsweredVia {
    // An unsupported-version Initial drew an RFC 9000 section 5.2.2 Version
    // Negotiation reply. No certificates are involved.
    VersionNegotiation,
    // The server presented its certificate and then rejected us for having none.
    // Carries SNI, so it survives an SNI-routing proxy.
    TlsRejection,
    // The handshake completed, which is only reachable where the operator has
    // turned mutual auth off.
    Handshake,
    // An HTTPS request returned a response.
    Https,
}

// The strongest outcome an unauthenticated probe can observe is that a server
// answered. Completing the mTLS handshake needs an issued client certificate,
// which does not exist before login, so a rejected handshake and a Version
// Negotiation reply are equally good proof of a live listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ReachabilityOutcome {
    Answered { via: AnsweredVia, rtt_micros: u32 },
    NoRoute,
    Silent,
}

impl ReachabilityOutcome {
    pub fn answered(&self) -> bool {
        matches!(self, Self::Answered { .. })
    }

    pub fn rtt_micros(&self) -> Option<u32> {
        match self {
            Self::Answered { rtt_micros, .. } => Some(*rtt_micros),
            _ => None,
        }
    }
}
