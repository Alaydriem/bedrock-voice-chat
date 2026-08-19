use std::path::{Path, PathBuf};

use serde::Deserialize;

fn default_dimension() -> String {
    "overworld".to_string()
}

// The BDS addon advertises every 5 ticks. Matching it keeps a staged roster fresh
// against the server's 15 s presence TTL with room for a missed post.
fn default_position_hz() -> u32 {
    4
}

// The server's own `voice.spatial_audio.broadcast_range` default. Distances are
// derived from it, so a server that changed the value needs it changed here too or
// players staged as near will land in the far tier.
fn default_voice_range() -> f32 {
    48.0
}

fn default_origin_y() -> f32 {
    64.0
}

// Set rather than absent, and the same for everyone in the scene.
//
// Chat delivery matches a player's own world against the chat room's, so an unset world means
// chat reaches nobody. Sharing one value costs nothing on the proximity side: the scope rule
// only rejects a pair when both declare a world and the two differ.
fn default_world_uuid() -> Option<String> {
    Some("8f14e45f-ea8f-4b62-9f2a-1c0d7e3b4a55".to_string())
}

fn default_world_name() -> String {
    "Overworld Survival".to_string()
}

// Fast enough that a held scene fills a chat dock while a shot is framed, slow enough to read.
fn default_chat_period_ms() -> u64 {
    2_500
}

fn default_group_name() -> String {
    crate::scene::ChannelNames::default_group()
}

fn default_group_bots() -> usize {
    3
}

/// A staged scene: who appears, on which server, and under whose identity.
///
/// Scenarios that only place players need `server`, `access_token`, `observer` and
/// `players`. The admin identity and the client binary are read only by scenarios
/// that connect real clients, so a positions-only run never has to hold them.
#[derive(Debug, Clone, Deserialize)]
pub struct SceneConfig {
    /// Base URL of the BVC server the app is signed in to.
    pub server: String,

    /// Server CA cert (PEM) path. Omit when the server presents a publicly trusted cert.
    pub ca: Option<String>,

    /// The server's `minecraft.access_token`, sent as `Authorization: Bearer <token>`.
    pub access_token: String,

    /// The signed-in player this scene is composed for.
    ///
    /// Placed at the origin facing yaw 0, which is what makes every bearing below
    /// mean what it says. The feed answers nothing for an observer it cannot find in
    /// the world, so this name must match the app's gamertag exactly.
    pub observer: String,

    /// Display names, consumed in order by whichever scenario runs.
    ///
    /// Never generated: a screenshot carries these names to the store, so a scenario
    /// that wants more players than this list holds fails rather than inventing one.
    pub players: Vec<String>,

    #[serde(default = "default_dimension")]
    pub dimension: String,

    /// World identifier attached to every staged player, and the chat room's id.
    ///
    /// Shared by everyone in the scene, so it filters nobody out of proximity — the scope
    /// rule only rejects a pair when both sides declare a world and the two differ. Chat
    /// needs it: a line is delivered only to players whose world matches the room's.
    #[serde(default = "default_world_uuid")]
    pub world_uuid: Option<String>,

    /// The world's display name, which supplies the chat picker's label.
    #[serde(default = "default_world_name")]
    pub world_name: String,

    /// Milliseconds between simulated chat lines.
    #[serde(default = "default_chat_period_ms")]
    pub chat_period_ms: u64,

    #[serde(default = "default_position_hz")]
    pub position_hz: u32,

    #[serde(default = "default_voice_range")]
    pub voice_range: f32,

    #[serde(default = "default_origin_y")]
    pub origin_y: f32,

    /// Admin mTLS identity, for minting the players a group scenario connects.
    pub admin_cert: Option<String>,

    pub admin_key: Option<String>,

    /// Path to `bvc_client_e2e`, the headless client a group scenario spawns per member.
    pub client_bin: Option<String>,

    /// The channel a group scenario creates. Join it by this name from the app to see
    /// the group in the rail — the server only shows a group to its own members.
    #[serde(default = "default_group_name")]
    pub group_name: String,

    /// Members the tool connects. Joining from the app adds one more, so 3 here reads
    /// as a four-member group; set 2 for a group of exactly three including yourself.
    #[serde(default = "default_group_bots")]
    pub group_bots: usize,

    /// Directory holding this config, so a relative path in it resolves against the
    /// file rather than against wherever the tool was launched from.
    #[serde(skip)]
    config_dir: PathBuf,
}

impl SceneConfig {
    pub fn load(path: &str) -> Result<Self, anyhow::Error> {
        let file = Path::new(path);
        let body = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("reading scene config {}: {}", path, e))?;
        let mut config: Self = toml::from_str(&body)
            .map_err(|e| anyhow::anyhow!("parsing scene config {}: {}", path, e))?;
        config.config_dir = file
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        config.validate()?;
        Ok(config)
    }

    /// The names a scenario asked for, or an error naming the shortfall.
    pub fn take_names(&self, from: usize, count: usize) -> Result<Vec<String>, anyhow::Error> {
        let end = from + count;
        if end > self.players.len() {
            return Err(anyhow::anyhow!(
                "this scenario needs {} names but `players` holds {}; add {} more",
                end,
                self.players.len(),
                end - self.players.len()
            ));
        }
        Ok(self.players[from..end].to_vec())
    }

    pub fn ca_pem(&self) -> Result<Option<String>, anyhow::Error> {
        match &self.ca {
            Some(path) => {
                let resolved = self.resolve(path);
                let pem = std::fs::read_to_string(&resolved).map_err(|e| {
                    anyhow::anyhow!("reading ca {}: {}", resolved.display(), e)
                })?;
                Ok(Some(pem))
            }
            None => Ok(None),
        }
    }

    /// A configured path, made absolute against the config file's own directory.
    ///
    /// An absolute path is returned untouched. A relative one resolves against the
    /// config rather than the working directory, so a scene file and the certs beside
    /// it stay a unit however the tool is invoked — and a path copied from another
    /// checkout cannot silently pick up that checkout's files.
    pub fn resolve(&self, path: &str) -> PathBuf {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            return candidate.to_path_buf();
        }
        self.config_dir.join(candidate)
    }

    pub fn ca_path(&self) -> Option<PathBuf> {
        self.ca.as_deref().map(|p| self.resolve(p))
    }

    pub fn server_base(&self) -> &str {
        self.server.trim_end_matches('/')
    }

    /// The three paths a scenario needs before it can connect real clients, resolved and
    /// confirmed to exist.
    ///
    /// Reported together so an operator fixes the config once rather than three times,
    /// and resolved paths are quoted back: a cert that is present but wrong is the hard
    /// failure here, because the server rejects a client certificate its own CA did not
    /// issue during the TLS handshake, and that surfaces as a transport error naming
    /// nothing.
    pub fn client_identity(&self) -> Result<(PathBuf, PathBuf, PathBuf), anyhow::Error> {
        let fields = [
            ("admin_cert", &self.admin_cert),
            ("admin_key", &self.admin_key),
            ("client_bin", &self.client_bin),
        ];

        let missing: Vec<&str> = fields
            .iter()
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| *name)
            .collect();

        if !missing.is_empty() {
            return Err(anyhow::anyhow!(
                "this scenario connects real clients and needs {} in the scene config",
                missing.join(", ")
            ));
        }

        let mut resolved = Vec::with_capacity(fields.len());
        for (name, value) in fields {
            let configured = value.as_deref().expect("checked above");
            let path = self.resolve(configured);

            // `client_bin` is the same cargo artifact on every platform and only Windows
            // spells it with an extension. Accepting either keeps one scene file working
            // on a dev box and in a Linux container both.
            let found = if path.exists() {
                Some(path.clone())
            } else if name == "client_bin" {
                let windows = path.with_extension("exe");
                windows.exists().then_some(windows)
            } else {
                None
            };

            let Some(found) = found else {
                return Err(anyhow::anyhow!(
                    "{} does not exist at {}",
                    name,
                    path.display()
                ));
            };

            resolved.push(found);
        }

        let mut drain = resolved.into_iter();
        Ok((
            drain.next().expect("three resolved"),
            drain.next().expect("three resolved"),
            drain.next().expect("three resolved"),
        ))
    }

    fn validate(&self) -> Result<(), anyhow::Error> {
        if self.observer.trim().is_empty() {
            return Err(anyhow::anyhow!("`observer` must name the signed-in player"));
        }

        if self.position_hz == 0 {
            return Err(anyhow::anyhow!(
                "`position_hz` must be at least 1; the server drops a player 15 s after its last post"
            ));
        }

        // A staged name equal to the observer's would overwrite the observer's own
        // entry at the same cache key, and the feed would place them on their own ring.
        if self.players.iter().any(|name| name == &self.observer) {
            return Err(anyhow::anyhow!(
                "`players` contains the observer ({}); the observer is staged separately",
                self.observer
            ));
        }

        Ok(())
    }
}
