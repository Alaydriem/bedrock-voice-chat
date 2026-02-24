/// Errors that can occur during authentication
#[derive(Debug)]
pub enum AuthError {
    /// Player not found in database
    PlayerNotFound,
    /// Player is banished
    PlayerBanished,
    /// Database error
    DatabaseError(String),
    /// Certificate error
    CertificateError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::PlayerNotFound => write!(f, "Player not found in database"),
            AuthError::PlayerBanished => write!(f, "Player is banished"),
            AuthError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AuthError::CertificateError(msg) => write!(f, "Certificate error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}
