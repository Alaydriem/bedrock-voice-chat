#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SframeError {
    /// No key for the frame's sender and epoch: a non-member, or an epoch already expired.
    #[error("no key for this frame")]
    UnknownKey,
    #[error("frame was already received")]
    Replay,
    #[error("frame failed authentication")]
    Authentication,
    #[error("no epoch keys installed")]
    NoEpoch,
}
