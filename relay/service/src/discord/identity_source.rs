use super::error::DiscordError;
use super::oauth_client::DiscordOAuthClient;

// Where the identity of the person enrolling comes from. Enum delegation rather than
// a trait object, matching how this crate dispatches its other outbound dependencies.
pub enum IdentitySource {
    OAuth(DiscordOAuthClient),
    // A fixed answer, so the callback is testable without a browser or a live app.
    Fixed(String),
}

impl IdentitySource {
    pub async fn identify(&self, code: &str) -> Result<String, DiscordError> {
        match self {
            Self::OAuth(client) => client.identify(code).await,
            Self::Fixed(id) => Ok(id.clone()),
        }
    }
}
