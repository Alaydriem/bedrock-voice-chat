use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformEntry {
    pub url: String,
}
