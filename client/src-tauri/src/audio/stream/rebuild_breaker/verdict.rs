use std::time::Duration;

/// What to do about a stream that failed to build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebuildVerdict {
    /// Rebuild once more, after this delay.
    Retry { after: Duration, attempt: u32 },
    /// Stop. This device is not going to open.
    Open,
}
