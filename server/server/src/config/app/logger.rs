use serde::{Deserialize, Serialize};

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_out() -> String {
    "stdout".to_string()
}

/// Logger Configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Logger {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_out")]
    pub out: String,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            out: default_log_out(),
        }
    }
}
