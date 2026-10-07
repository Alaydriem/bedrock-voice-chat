use std::time::Duration;

/// Where releases are discovered and downloaded from. `Default` is the published GitHub
/// release channel.
#[derive(Debug, Clone)]
pub struct ReleaseSources {
    pub manifest_urls: Vec<String>,
    pub asset_base: String,
    pub manifest_timeout: Duration,
}

impl ReleaseSources {
    pub fn new(manifest_urls: Vec<String>, asset_base: String, manifest_timeout: Duration) -> Self {
        Self {
            manifest_urls,
            asset_base,
            manifest_timeout,
        }
    }
}

impl Default for ReleaseSources {
    fn default() -> Self {
        Self::new(
            vec![
                "https://alaydriem.github.io/bedrock-voice-chat/updater/latest.json".to_string(),
                "https://alaydriem.github.io/bedrock-voice-chat/updater/beta.json".to_string(),
            ],
            "https://github.com/Alaydriem/bedrock-voice-chat/releases/download".to_string(),
            Duration::from_secs(15),
        )
    }
}
