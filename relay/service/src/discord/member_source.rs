use super::bot_client::DiscordBotClient;
use super::error::DiscordError;

// A fixed answer, for tests and for a deployment that has not yet been given a
// guild.
pub struct FixedMemberSource {
    // `None` answers as if Discord could not be reached.
    roles: Option<Vec<String>>,
}

impl FixedMemberSource {
    pub fn new(roles: Vec<String>) -> Self {
        Self { roles: Some(roles) }
    }

    pub fn absent() -> Self {
        Self::new(Vec::new())
    }

    pub fn unreachable() -> Self {
        Self { roles: None }
    }

    pub fn role_ids(&self) -> Result<Vec<String>, DiscordError> {
        self.roles
            .clone()
            .ok_or_else(|| DiscordError::Http("discord is unreachable".to_string()))
    }
}

// Where a member's roles come from. Enum delegation rather than a trait object,
// matching how the server dispatches its own providers.
pub enum MemberSource {
    Bot(DiscordBotClient),
    Fixed(FixedMemberSource),
}

impl MemberSource {
    pub async fn role_ids(&self, discord_user_id: &str) -> Result<Vec<String>, DiscordError> {
        match self {
            Self::Bot(client) => client.role_ids(discord_user_id).await,
            Self::Fixed(fixed) => fixed.role_ids(),
        }
    }
}
