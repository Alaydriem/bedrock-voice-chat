use serde::{Deserialize, Serialize};

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_path() -> String {
    "./logs".to_string()
}

/// Logger Configuration
#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Logger {
    #[serde(default = "default_log_level")]
    pub level: String,
    // Directory for the rotating JSON log, created if missing. The console sink
    // is unconditional and does not depend on this.
    #[serde(default = "default_log_path")]
    pub path: String,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            path: default_log_path(),
        }
    }
}
