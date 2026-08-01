// Why the upgrade was refused. Returned instead of accepting and then going silent, because a
// client that completes a handshake and receives nothing has no way to tell a wrong key from a
// server with nothing to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    MissingKey,
    InvalidKey,
}

impl RejectReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingKey => "missing authentication key",
            Self::InvalidKey => "invalid authentication key",
        }
    }
}
