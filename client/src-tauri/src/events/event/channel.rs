use log::info;
use serde::{Deserialize, Serialize};
use tauri::Listener;

pub(crate) const CHANNEL_EVENT: &str = "channel_event";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ChannelEventPayload {
    // One of "create", "delete", "join" or "leave"
    pub event_type: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub creator: Option<String>,
    pub player_name: String,
    pub timestamp: Option<i64>,
}

impl ChannelEventPayload {
    pub fn new(
        event_type: String,
        channel_id: String,
        channel_name: Option<String>,
        creator: Option<String>,
        player_name: String,
        timestamp: Option<i64>,
    ) -> Self {
        Self {
            event_type,
            channel_id,
            channel_name,
            creator,
            player_name,
            timestamp,
        }
    }

    #[allow(unused)]
    pub fn register(app: &tauri::App) {
        app.listen(CHANNEL_EVENT, |event| {
            if let Ok(payload) = serde_json::from_str::<ChannelEventPayload>(&event.payload()) {
                info!(
                    "Channel event: {} {} in channel {}",
                    payload.player_name, payload.event_type, payload.channel_id
                );
            }
        });
    }
}
