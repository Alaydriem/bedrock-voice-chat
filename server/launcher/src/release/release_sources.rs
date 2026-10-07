use std::time::Duration;

use super::ReleaseChannel;

/// Where releases are discovered and downloaded from. `Default` is GitHub, on the channel the
/// launcher was built for.
#[derive(Debug, Clone)]
pub struct ReleaseSources {
    pub updater_base: String,
    pub asset_base: String,
    pub manifest_timeout: Duration,
    pub channel: ReleaseChannel,
}

impl ReleaseSources {
    pub fn new(
        updater_base: String,
        asset_base: String,
        manifest_timeout: Duration,
        channel: ReleaseChannel,
    ) -> Self {
        Self {
            updater_base,
            asset_base,
            manifest_timeout,
            channel,
        }
    }

    pub fn manifest_url(&self) -> String {
        format!("{}/{}", self.updater_base, self.channel.manifest_file())
    }
}

impl Default for ReleaseSources {
    fn default() -> Self {
        Self::new(
            "https://www.bedrockvoicechat.com/updater".to_string(),
            "https://github.com/Alaydriem/bedrock-voice-chat/releases/download".to_string(),
            Duration::from_secs(15),
            ReleaseChannel::built(),
        )
    }
}
