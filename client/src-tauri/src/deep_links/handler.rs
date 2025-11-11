use common::structs::DeepLink;
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;
use tauri_plugin_log::log::{info, error};

/// Trait for handling deep link events
pub trait DeepLinkHandler {
    /// Handle a deep link event
    fn handle(&self, app: &AppHandle) -> Result<(), String>;
}

impl DeepLinkHandler for DeepLink {
    fn handle(&self, app: &AppHandle) -> Result<(), String> {
        log::info!("Rust: Handling deep link: {}", self.url);

        // Clone what we need for the async block
        let app_handle = app.clone();
        let deep_link = self.clone();

        // Spawn async work to avoid blocking the callback thread
        tauri::async_runtime::spawn(async move {
            match app_handle.store("store.json") {
                Ok(store) => {
                    store.set("pending_deep_link", serde_json::json!(deep_link.url.clone()));
                    match store.save() {
                        Ok(_) => {
                            if let Err(e) = app_handle.emit("deep-link-received", deep_link) {
                                error!("Failed to emit deep-link-received event: {}", e);
                            } else {
                                info!("Rust: Emitted deep-link-received event");
                            }
                        }
                        Err(e) => {
                            error!("Failed to save store: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get store: {}", e);
                }
            }
        });

        Ok(())
    }
}
