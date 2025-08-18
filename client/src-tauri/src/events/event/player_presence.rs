use log::info;
use serde::{Deserialize, Serialize};
use tauri::Listener;

// Events Tauri is to subscribe to
// This is the general notification event
pub(crate) const PLAYER_PRESENCE: &str = "player_presence";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Presence {
    pub player: String,
    pub status: String,
}

impl Presence {
    pub fn new(player: String, status: String) -> Self {
        Self { player, status }
    }

    #[allow(unused)]
    pub fn register(app: &tauri::App) {
        app.listen(PLAYER_PRESENCE, |event| {
            if let Ok(payload) = serde_json::from_str::<Presence>(&event.payload()) {
                info!("Player {} is now {}", payload.player, payload.status);
            }
        });
    }
}
