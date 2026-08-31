#[derive(Debug, thiserror::Error)]
pub enum PairingCodeError {
    #[error("a stored pairing digest is 64 hex characters; this value is not")]
    Digest,
}
