#[derive(Debug, thiserror::Error)]
pub enum DiscordLinkError {
    #[error("discord linking is not configured")]
    NotConfigured,
    #[error("oauth state mismatch")]
    StateMismatch,
    #[error("no access token in redirect")]
    TokenMissing,
    #[error("oauth window closed before completion")]
    WindowClosed,
    #[error("discord http error: {0}")]
    Http(String),
}
