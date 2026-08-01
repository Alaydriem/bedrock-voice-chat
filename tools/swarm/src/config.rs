use std::path::Path;

use serde::Deserialize;

use crate::lxd_config::LxdConfig;
use crate::target_spec::TargetSpec;

fn default_prefix() -> String {
    "SwarmBot".to_string()
}

fn default_group_size() -> usize {
    5
}

fn default_duration() -> u64 {
    120
}

// The BDS addon posts positions every 5 ticks. Matching it means the swarm
// advertises at the real cadence rather than a convenient one.
fn default_position_hz() -> u32 {
    4
}

// Every position payload in production carries a world identifier, and it is
// roughly half of a player's encoded size. Omitting it would understate
// datagram size by ~2x and hide exactly the pressure this run exists to measure.
fn default_world_uuid() -> String {
    "8f14e45f-ea8f-4b62-9f2a-1c0d7e3b4a55".to_string()
}

/// The single config file that drives an entire swarm run. Everything the
/// controller and minter need lives here so a run is `swarm controller
/// --config swarm.toml` with no other flags.
#[derive(Debug, Clone, Deserialize)]
pub struct SwarmConfig {
    /// Base URL of the target BVC server, e.g. `https://bvc.home.lan:8444`.
    pub server: String,
    /// Path to the server CA cert (PEM). Needed when the server uses a private
    /// CA; its contents are embedded into each agent job so containers need no
    /// cert files. Omit if the server presents a publicly trusted cert.
    pub ca: Option<String>,
    /// Admin mTLS identity used for minting: cert PEM path.
    pub admin_cert: String,
    /// Admin mTLS identity used for minting: private key PEM path.
    pub admin_key: String,
    /// The server's Minecraft mod access token, sent as `X-MC-Access-Token`
    /// when agents POST bot positions to `/api/position`.
    pub access_token: String,
    /// Bot gamertag prefix; the i-th bot is `{prefix}{i:03}`.
    #[serde(default = "default_prefix")]
    pub prefix: String,
    /// Bots per voice group (container-local). Larger groups fan out more audio.
    #[serde(default = "default_group_size")]
    pub group_size: usize,
    /// Seconds each bot streams audio.
    #[serde(default = "default_duration")]
    pub duration_secs: u64,
    /// Simulated realm population: how many players the position feed advertises
    /// in a single request.
    ///
    /// This is the axis that drives position datagram size, and it is
    /// independent of how many bots are on voice. The game mod posts EVERY
    /// player on the realm in one request, so a realm of 100 with 4 voice
    /// clients — the common shape — is expressed as `realm_players = 100` with 4
    /// bots. Bots always appear in the roster; any remainder is synthetic
    /// filler that never connects. Defaults to the bot count.
    pub realm_players: Option<usize>,
    /// How often the full roster is advertised, in Hz. Defaults to the game
    /// mod's real cadence.
    #[serde(default = "default_position_hz")]
    pub position_hz: u32,
    /// World identifier attached to every advertised player, matching what a
    /// real mod publishes. Length matters: it is a large share of per-player
    /// encoded size.
    #[serde(default = "default_world_uuid")]
    pub world_uuid: String,
    /// Path (on the controller) to the prebuilt Linux `bvc_client_e2e` binary,
    /// pushed into each container.
    pub client_bin: String,
    /// Path (on the controller) to the prebuilt Linux `swarm` binary, pushed
    /// into each container to run the in-container agent.
    pub swarm_bin: String,
    /// LXD-wide settings (client identity, image, cloud-init).
    pub lxd: LxdConfig,
    /// Participating LXD hosts.
    pub target: Vec<TargetSpec>,
}

impl SwarmConfig {
    pub fn load(path: &str) -> Result<Self, anyhow::Error> {
        let text = std::fs::read_to_string(Path::new(path))
            .map_err(|e| anyhow::anyhow!("reading config {}: {}", path, e))?;
        let config: SwarmConfig =
            toml::from_str(&text).map_err(|e| anyhow::anyhow!("parsing config {}: {}", path, e))?;
        if config.target.is_empty() {
            return Err(anyhow::anyhow!("config has no [[target]] entries"));
        }
        if config.group_size == 0 {
            return Err(anyhow::anyhow!("group_size must be > 0"));
        }
        for t in &config.target {
            if t.bots_per_container == 0 || t.containers == 0 {
                return Err(anyhow::anyhow!(
                    "target {} must have containers > 0 and bots_per_container > 0",
                    t.name
                ));
            }
        }
        if config.position_hz == 0 {
            return Err(anyhow::anyhow!("position_hz must be > 0"));
        }
        if let Some(realm) = config.realm_players {
            let bots = config.total_bots();
            if realm < bots {
                return Err(anyhow::anyhow!(
                    "realm_players ({realm}) is below total bots ({bots}); every bot must appear \
                     in the advertised roster"
                ));
            }
        }
        Ok(config)
    }

    pub fn total_bots(&self) -> usize {
        self.target.iter().map(|t| t.total_bots()).sum()
    }

    /// Advertised realm population, defaulting to the bot count.
    pub fn realm_size(&self) -> usize {
        self.realm_players.unwrap_or_else(|| self.total_bots())
    }

    /// Deterministic gamertag for the i-th bot across the whole swarm.
    pub fn bot_name(&self, index: usize) -> String {
        format!("{}{:03}", self.prefix, index)
    }

    /// Gamertag for a synthetic filler player. Prefixed distinctly so a filler
    /// can never collide with a minted bot, and shaped like a real gamertag so
    /// its encoded length is representative.
    pub fn filler_name(&self, index: usize) -> String {
        format!("{}Idle{:03}", self.prefix, index)
    }
}
