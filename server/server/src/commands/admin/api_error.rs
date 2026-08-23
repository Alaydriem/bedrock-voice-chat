use reqwest::StatusCode;

#[derive(Debug)]
pub enum AdminApiError {
    NotFound,
    Conflict,
    Forbidden,
    BadRequest(String),
    Unexpected(StatusCode, String),
    Transport(anyhow::Error),
}

impl std::fmt::Display for AdminApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminApiError::NotFound => write!(f, "not found"),
            AdminApiError::Conflict => write!(f, "conflict"),
            AdminApiError::Forbidden => write!(f, "forbidden"),
            AdminApiError::BadRequest(b) => write!(f, "bad request: {}", b),
            AdminApiError::Unexpected(s, b) => write!(f, "unexpected status {}: {}", s, b),
            AdminApiError::Transport(e) => write!(f, "transport error: {}", e),
        }
    }
}

impl std::error::Error for AdminApiError {}

impl From<anyhow::Error> for AdminApiError {
    fn from(e: anyhow::Error) -> Self {
        AdminApiError::Transport(e)
    }
}
