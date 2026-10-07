use std::fs;
use std::path::Path;

/// BDS `server.properties`.
pub struct ServerProperties;

impl ServerProperties {
    const LEVEL_NAME_KEY: &'static str = "level-name=";
    const DEFAULT_LEVEL_NAME: &'static str = "Bedrock level";

    /// The world directory name under `worlds/`. BDS uses `Bedrock level` when unset.
    pub fn level_name(path: &Path) -> String {
        fs::read_to_string(path)
            .ok()
            .and_then(|content| {
                content.lines().find_map(|line| {
                    line.trim_start()
                        .strip_prefix(Self::LEVEL_NAME_KEY)
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty())
                })
            })
            .unwrap_or_else(|| Self::DEFAULT_LEVEL_NAME.to_string())
    }
}
