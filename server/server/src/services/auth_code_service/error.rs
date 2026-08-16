#[derive(Debug)]
pub enum AuthCodeError {
    CodeNotFound,
    CodeExpired,
    CodeAlreadyUsed,
    PlayerNotFound,
    DatabaseError(String),
}

impl std::fmt::Display for AuthCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthCodeError::CodeNotFound => write!(f, "Auth code not found"),
            AuthCodeError::CodeExpired => write!(f, "Auth code has expired"),
            AuthCodeError::CodeAlreadyUsed => write!(f, "Auth code has already been used"),
            AuthCodeError::PlayerNotFound => write!(f, "Player not found for auth code"),
            AuthCodeError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for AuthCodeError {}
