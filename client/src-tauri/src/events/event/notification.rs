use log::{info, warn, error};
use serde::{Serialize, Deserialize};
use tauri::Listener;

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
    pub fn new(title: String, body: String, level: Option<String>,  e: Option<String>, icon: Option<String>, sound: Option<String>) -> Self {
        Self { title, body, level, e, icon, sound }
    }

    pub fn register(app: &tauri::App) {
        app.listen(EVENT_NOTIFICATION, |event| {
            if let Ok(payload) = serde_json::from_str::<Notification>(&event.payload()) {
                match payload.level {
                    Some(level) => match level.as_str() {
                        "info" => info!("{}: {}", payload.title, payload.body),
                        "warn" => warn!("{}: {}", payload.title, payload.body),
                        "error" => error!("{}: {} {:?}", payload.title, payload.body, payload.e),
                        _ => info!("{}: {}", payload.title, payload.body),
                    },
                    None => info!("{}: {}", payload.title, payload.body),
                };
            }
        });
    }
}