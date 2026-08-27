use super::bot_client::DiscordBotClient;
use super::error::DiscordError;

// A fixed answer, for tests and for a deployment that has not yet been given a
// guild.
pub struct FixedMemberSource {
    roles: Vec<String>,
}

impl FixedMemberSource {
    pub fn new(roles: Vec<String>) -> Self {
        Self { roles }
    }

    pub fn absent() -> Self {
        Self { roles: Vec::new() }
    }

    pub fn role_ids(&self) -> Vec<String> {
        self.roles.clone()
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
            Self::Fixed(fixed) => Ok(fixed.role_ids()),
        }
    }
}
