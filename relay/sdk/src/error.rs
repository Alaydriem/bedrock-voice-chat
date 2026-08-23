// What can go wrong, as the consumer sees it.
//
// Deliberately coarse: a bridge can act on "not connected" and on "your config
// is wrong", and nothing else here is actionable from Kotlin. The detail is in
// the message and the logs.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SdkError {
    #[error("the peer link could not be used: {reason}")]
    PeerLink { reason: String },
    #[error("the session could not be opened: {reason}")]
    Open { reason: String },
    #[error("not connected")]
    NotConnected,
}
