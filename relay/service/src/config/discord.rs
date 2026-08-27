use serde::{Deserialize, Serialize};

// Which Discord memberships qualify for an assigned name.
//
// A configured set of role ids rather than a tier model: which memberships qualify
// is a deployment decision, so it changes with a config edit and a restart rather
// than with a release. Patreon and YouTube both sync into Discord roles, so the
// distinction between them never reaches this code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub guild_id: String,
    pub bot_token: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub qualifying_role_ids: Vec<String>,
}

impl DiscordConfig {
    // An empty configured set refuses everyone. Reading it as "no restriction"
    // would turn a misconfiguration into an open registry.
    pub fn qualifies(&self, role_ids: &[String]) -> bool {
        self.qualifying_role_ids
            .iter()
            .any(|wanted| role_ids.iter().any(|held| held == wanted))
    }
}
