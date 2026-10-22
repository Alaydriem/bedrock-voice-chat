/// Why this client is no longer in a protected group, or never got in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LostReason {
    /// The checking member pressed Deny, or the check failed.
    Refused,
    /// The joiner said the codes do not match.
    Mismatch,
    /// No member answered, or the check ran out of time.
    Expired,
    NoAdmitter,
    SocketClosed,
    ProtocolFailure,
}

impl LostReason {
    /// The token the webview maps to its copy.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Mismatch => "mismatch",
            Self::Expired => "expired",
            Self::NoAdmitter => "no_admitter",
            Self::SocketClosed => "socket_closed",
            Self::ProtocolFailure => "protocol_failure",
        }
    }
}
