/// What the rate limiter decided about one inbound frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateVerdict {
    Within,
    Over,
    /// Over the limit in every second for longer than the grace period: close the socket.
    Close,
}
