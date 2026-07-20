use serde::{Deserialize, Serialize};

/// The unit of work handed to one `swarm agent`, delivered as a JSON document on
/// the agent's stdin. Carrying codes, the access token, and CA contents here
/// (rather than argv) keeps secrets out of `ps` and means remote hosts need no
/// cert files of their own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentJob {
    pub server: String,
    pub access_token: String,
    /// Server CA cert PEM contents (not a path), or None for a publicly trusted server.
    pub ca_pem: Option<String>,
    pub group_size: usize,
    /// Global index of this host's first bot, so gamertags/positions never collide.
    pub offset: usize,
    pub duration_secs: u64,
    /// `(gamertag, login_code)` for exactly this host's bots, in offset order.
    pub codes: Vec<(String, String)>,
}
