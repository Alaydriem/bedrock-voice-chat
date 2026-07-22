use serde::Deserialize;

fn default_containers() -> usize {
    1
}

/// One physical LXD host in the swarm. The controller launches `containers`
/// ephemeral containers on it, each running the agent for `bots_per_container`
/// bots. `containers = 1` is the per-host model; set `bots_per_container` to
/// `group_size` for one voice group per container.
#[derive(Debug, Clone, Deserialize)]
pub struct TargetSpec {
    /// Label used in the report and container names.
    pub name: String,
    /// LXD HTTPS API endpoint, e.g. `https://192.168.1.10:8443`.
    pub endpoint: String,
    /// The daemon's server cert PEM (path) for TLS trust. When omitted the
    /// controller accepts the daemon's self-signed cert (LAN convenience).
    pub server_cert: Option<String>,
    #[serde(default = "default_containers")]
    pub containers: usize,
    pub bots_per_container: usize,
}

impl TargetSpec {
    pub fn total_bots(&self) -> usize {
        self.containers * self.bots_per_container
    }
}
