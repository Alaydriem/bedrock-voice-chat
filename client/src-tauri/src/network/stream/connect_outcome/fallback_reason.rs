/// Why a connect tried a transport the reachability probe did not choose.
///
/// The probe's verdict and the session's transport agree on most networks. Where they differ,
/// the direction of the disagreement is the diagnosis: a QUIC walk that reached nothing says
/// the network filters UDP, and a WebSocket choice rescued by QUIC says the probe misread it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// The chosen transport carried the session. Nothing else was dialled.
    None,
    /// Every QUIC candidate was walked and none carried, so the WebSocket was dialled.
    QuicFailed,
    /// A QUIC session on this host had already degraded, so the walk was skipped outright.
    /// Distinct from `QuicFailed`: nothing was measured on this attempt, and a walk that
    /// never ran is not evidence that UDP is blocked.
    Demoted,
    /// The WebSocket was chosen, did not carry, and the QUIC plan was walked instead.
    WebSocketFailed,
}

impl FallbackReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::QuicFailed => "quic_failed",
            Self::Demoted => "demoted",
            Self::WebSocketFailed => "websocket_failed",
        }
    }
}
