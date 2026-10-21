use common::structs::DeepLink;
use tauri::{AppHandle, Emitter};
use tauri_plugin_curia::curia;
use tauri_plugin_store::StoreExt;

/// Trait for handling deep link events
pub trait DeepLinkHandler {
    /// Handle a deep link event
    fn handle(&self, app: &AppHandle) -> Result<(), String>;
}

impl DeepLinkHandler for DeepLink {
    fn handle(&self, app: &AppHandle) -> Result<(), String> {
        curia::info!("handling deep link", {
            url: crate::deep_links::UrlRedactor::for_log(&self.url).to_string(),
        });

        // Clone what we need for the async block
        let app_handle = app.clone();
        let deep_link = self.clone();

        // Spawn async work to avoid blocking the callback thread
        tauri::async_runtime::spawn(async move {
            match app_handle.store("store.json") {
                Ok(store) => {
                    store.set(
                        "pending_deep_link",
                        serde_json::json!(crate::deep_links::UrlRedactor::for_storage(
                            &deep_link.url
                        )),
                    );
                    match store.save() {
                        Ok(_) => {
                            if let Err(e) = app_handle.emit("deep-link-received", deep_link) {
                                curia::error!("failed to emit the deep-link-received event", {
                                    error: e.to_string(),
                                });
                            } else {
                                curia::info!("emitted the deep-link-received event");
                            }
                        }
                        Err(e) => {
                            curia::error!("failed to save the store for a deep link", {
                                error: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    curia::error!("failed to get the store for a deep link", {
                        error: e.to_string(),
                    });
                }
            }
        });

        Ok(())
    }
}
