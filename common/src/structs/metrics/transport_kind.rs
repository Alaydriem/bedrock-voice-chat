use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Which transport is carrying a voice session.
///
/// Every diagnostic, metric and error message that describes a link needs this: the two
/// transports have different latency characteristics and different failure modes, so a
/// measurement without it cannot be compared against another, and an error code that says
/// only "the voice link failed" cannot say which one did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum TransportKind {
    /// QUIC datagrams. Unreliable delivery, no head-of-line blocking, the default.
    Quic,
    /// Application packets over a TLS WebSocket. Reliable and ordered, which costs latency
    /// under loss — taken only where QUIC does not arrive at all.
    WebSocket,
}

impl TransportKind {
    /// The stable label used in metric dimensions and log fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Quic => "quic",
            Self::WebSocket => "websocket",
        }
    }
}

impl std::fmt::Display for TransportKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
