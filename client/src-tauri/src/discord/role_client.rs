use common::errors::DiscordLinkError;

pub struct DiscordRoleClient;

impl DiscordRoleClient {
    pub async fn fetch_role_ids(
        http: &reqwest::Client,
        access_token: &str,
        guild_id: &str,
    ) -> Result<Vec<String>, DiscordLinkError> {
        let url = format!(
            "https://discord.com/api/v10/users/@me/guilds/{}/member",
            guild_id
        );
        let resp = http
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| DiscordLinkError::Http(e.to_string()))?;

        // Not a member of the guild → no roles, feature stays locked.
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if !resp.status().is_success() {
            return Err(DiscordLinkError::Http(format!("status {}", resp.status())));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DiscordLinkError::Http(e.to_string()))?;
        Ok(Self::roles_from_member(&body))
    }

    pub fn roles_from_member(body: &serde_json::Value) -> Vec<String> {
        body.get("roles")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    }
}
