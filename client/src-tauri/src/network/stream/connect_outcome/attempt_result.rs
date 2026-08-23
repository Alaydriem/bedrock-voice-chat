/// Why one candidate in the connect walk did not carry the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttemptResult {
    /// The handshake completed. At most one candidate reports this.
    Connected,
    /// Nothing answered inside the candidate's budget, which is what a blackholed UDP port
    /// looks like — the distinguishing signature of a network that filters rather than
    /// refuses.
    TimedOut,
    /// The endpoint answered and refused. The path works; something above it did not.
    Rejected,
}

impl AttemptResult {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::TimedOut => "timed_out",
            Self::Rejected => "rejected",
        }
    }
}
