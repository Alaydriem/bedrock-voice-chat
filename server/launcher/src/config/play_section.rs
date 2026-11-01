use serde::Deserialize;

/// The `server.tls.play` block. Present, even empty, it selects play mode.
#[derive(Debug, Default, Deserialize)]
pub struct PlaySection {
    #[serde(default)]
    pub address: Option<String>,
}
