use serde::{Deserialize, Serialize};
use tauri::Listener;
use tauri_plugin_curia::curia;

// Events Tauri is to subscribe to
// This is the general notification event
pub(crate) const EVENT_NOTIFICATION: &str = "notification";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Notification {
    pub title: String,
    pub body: String,
    pub level: Option<String>,
    pub e: Option<String>,
    pub icon: Option<String>,
    pub sound: Option<String>,
}

impl Notification {
    pub fn new(
        title: String,
        body: String,
        level: Option<String>,
        e: Option<String>,
        icon: Option<String>,
        sound: Option<String>,
    ) -> Self {
        Self {
            title,
            body,
            level,
            e,
            icon,
            sound,
        }
    }

    pub fn register(app: &tauri::App) {
        app.listen(EVENT_NOTIFICATION, |event| {
            if let Ok(payload) = serde_json::from_str::<Notification>(&event.payload()) {
                match payload.level.as_deref() {
                    Some("warn") => curia::warn!("webview notification", {
                        title: payload.title,
                        body: payload.body,
                    }),
                    Some("error") => curia::error!("webview notification", {
                        title: payload.title,
                        body: payload.body,
                        error: payload.e,
                    }),
                    _ => curia::info!("webview notification", {
                        title: payload.title,
                        body: payload.body,
                    }),
                };
            }
        });
    }
}
