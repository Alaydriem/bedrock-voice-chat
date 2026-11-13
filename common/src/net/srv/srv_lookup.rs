use super::SrvAnswer;

/// What one lookup step found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SrvLookup {
    // The server answered with SRV records.
    Found(Vec<SrvAnswer>),
    // The server answered, and has no record. This is final.
    NoRecord,
    // No usable answer came back: a timeout, a socket error, or a server error.
    Failed(String),
}
