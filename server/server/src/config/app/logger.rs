use serde::{Deserialize, Serialize};

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_out() -> String {
    "stdout".to_string()
}

/// Logger Configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigLogger {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_out")]
    pub out: String,
}

impl Default for ApplicationConfigLogger {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            out: default_log_out(),
        }
    }
}
