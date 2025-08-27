use common::structs::packet::ServerErrorType;
use log::error;
use serde::{Deserialize, Serialize};
use tauri::Listener;

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

    #[allow(unused)]
    pub fn register(app: &tauri::App) {
        app.listen(SERVER_ERROR, |event| {
            if let Ok(payload) = serde_json::from_str::<ServerError>(&event.payload()) {
                error!(
                    "Server error occurred: {:?} - {}",
                    payload.error_type, payload.message
                );
            }
        });
    }
}
