#[derive(Debug, thiserror::Error)]
pub enum DiscordError {
    #[error("discord request failed: {0}")]
    Http(String),
    #[error("discord returned status {0}")]
    Status(u16),
}
