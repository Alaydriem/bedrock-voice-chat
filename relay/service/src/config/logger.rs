use serde::{Deserialize, Serialize};

fn default_level() -> String {
    "info".to_string()
}

fn default_path() -> String {
    "/var/log/bvc-relay".to_string()
}

// Where the registry's operational output goes.
//
// Not a contradiction of the registry keeping no durable files: a log is what the
// process did, not state it depends on. Losing the directory costs history and nothing
// else, which is why an unwritable one degrades to console rather than stopping the
// start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    // `error`, `warn`, `info`, `debug` or `trace`. `RUST_LOG` overrides it wholesale
    // when a specific target needs chasing.
    #[serde(default = "default_level")]
    pub level: String,
    // Directory for the rotating JSON log, created if missing. The console sink is
    // unconditional and does not depend on this.
    #[serde(default = "default_path")]
    pub path: String,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: default_level(),
            path: default_path(),
        }
    }
}

impl LoggerConfig {
    // Directives rather than a bare level, so the noisiest dependencies can be held
    // down without silencing the registry's own lines. iroh and its transport log
    // heavily below `info` and bury everything an operator needs.
    pub fn directives(&self) -> String {
        match self.level.to_lowercase().as_str() {
            "trace" => "trace,iroh=debug,iroh_quinn=debug,netwatch=debug".to_string(),
            "debug" => "debug,iroh=info,iroh_quinn=info,netwatch=info".to_string(),
            "warn" => "warn".to_string(),
            "error" => "error".to_string(),
            _ => "info,iroh=warn,iroh_quinn=warn,netwatch=warn".to_string(),
        }
    }
}
