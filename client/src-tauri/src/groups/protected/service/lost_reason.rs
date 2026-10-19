/// Why this client is no longer in a protected group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LostReason {
    WrongPassword,
    NoAdmitter,
    AdmitterUnreachable,
    SocketClosed,
    ProtocolFailure,
}

impl LostReason {
    /// The token the webview maps to its copy.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WrongPassword => "wrong_password",
            Self::NoAdmitter => "no_admitter",
            Self::AdmitterUnreachable => "admitter_unreachable",
            Self::SocketClosed => "socket_closed",
            Self::ProtocolFailure => "protocol_failure",
        }
    }
}
