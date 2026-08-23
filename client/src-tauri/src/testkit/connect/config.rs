// the standalone boot smoke.
#[derive(Debug, Clone)]
pub struct ConnectConfig {
    pub server: String,
    pub gamertag: String,
    pub code: String,
    // Channel display name — Connector calls create_channel and joins the returned id.
    pub channel: Option<String>,
    // Pre-existing channel id — Connector skips create_channel and joins directly.
    // Takes precedence over `channel` when both are set.
    pub channel_id: Option<String>,
}
