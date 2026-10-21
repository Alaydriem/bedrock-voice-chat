use common::structs::packet::ServerErrorType;
use serde::{Deserialize, Serialize};
use tauri::Listener;
use tauri_plugin_curia::curia;

pub(crate) const SERVER_ERROR: &str = "server_error";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ServerError {
    pub error_type: ServerErrorType,
    pub message: String,
}

impl ServerError {
    pub fn new(error_type: ServerErrorType, message: String) -> Self {
        Self {
            error_type,
            message,
        }
    }

    pub fn register(app: &tauri::App) {
        app.listen(SERVER_ERROR, |event| {
            if let Ok(payload) = serde_json::from_str::<ServerError>(&event.payload()) {
                curia::error!("server error occurred", {
                    error_type: format!("{:?}", payload.error_type),
                    error: payload.message,
                });
            }
        });
    }
}
