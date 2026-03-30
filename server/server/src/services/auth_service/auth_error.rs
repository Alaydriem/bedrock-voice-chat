#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("player not found in database")]
    PlayerNotFound,
    #[error("player is banished")]
    PlayerBanished,
    #[error("database error: {0}")]
    DatabaseError(String),
    #[error("certificate error: {0}")]
    CertificateError(String),
}
