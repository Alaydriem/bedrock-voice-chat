use serde_json::Value;

use crate::config::DiscordConfig;

use super::error::DiscordError;

// Reads a member's guild roles with the relay's own bot credential.
//
// Not the member's OAuth token: the implicit flow the web UI uses issues a
// short-lived token with no refresh, so a daily re-check against it would mean
// re-authenticating every operator every day. The bot is already resident in the
// guild and its credential does not expire.
pub struct DiscordBotClient {
    http: reqwest::Client,
    guild_id: String,
    bot_token: String,
}

impl DiscordBotClient {
    pub fn new(config: &DiscordConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            guild_id: config.guild_id.clone(),
            bot_token: config.bot_token.clone(),
        }
    }

    pub async fn role_ids(&self, discord_user_id: &str) -> Result<Vec<String>, DiscordError> {
        let url = format!(
            "https://discord.com/api/v10/guilds/{}/members/{}",
            self.guild_id, discord_user_id
        );

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| DiscordError::Http(e.to_string()))?;

        // Not a member of the guild. No roles, and not an error: a departure and a
        // cancelled membership are the same observation here, and neither is an
        // outage.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }

        if !response.status().is_success() {
            return Err(DiscordError::Status(response.status().as_u16()));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| DiscordError::Http(e.to_string()))?;

        Ok(Self::roles_from_member(&body))
    }

    pub fn roles_from_member(body: &Value) -> Vec<String> {
        body.get("roles")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}
