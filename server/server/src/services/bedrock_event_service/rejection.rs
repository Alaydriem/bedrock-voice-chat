#[derive(Debug, Clone)]
pub enum BedrockEventRejection {
    BdsHealthy,
    NotFound,
    Internal(String),
}

impl std::fmt::Display for BedrockEventRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BdsHealthy => write!(
                f,
                "BDS addon is healthy for this world; proxy event rejected"
            ),
            Self::NotFound => write!(f, "Target event or block not found"),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}
