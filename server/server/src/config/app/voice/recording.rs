use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// Whether this server permits clients to record voice sessions.
///
/// The recording itself is written on the player's own machine and never reaches
/// the server, so this states a policy the stock client honours rather than a
/// boundary the server can hold.
#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct RecordingConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
        }
    }
}
