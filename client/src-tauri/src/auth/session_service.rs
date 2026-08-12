use crate::analytics::AnalyticsService;
use crate::keyring::KeyringService;
use crate::structs::app_state::AppState;
use common::structs::ServerListEntry;
use tauri_plugin_store::StoreExt;

/// Ending a session with a server, and forgetting that it was ever signed into.
///
/// Behind a service because two callers need it and a Tauri command is a delegator: the user
/// pressing sign out, and a connect that failed because the credentials on this device cannot
/// be accepted. The second has no user to press anything, and must not be a second copy of
/// what the first does.
pub(crate) struct SessionService {
    app_handle: tauri::AppHandle,
}

impl SessionService {
    pub(crate) fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }

    /// Clears every trace of the current server: its keyring credentials, its API client, the
    /// current session pointers, and its entry in the saved server list.
    pub(crate) async fn forget_current_server(
        &self,
        state: &mut AppState,
        keyring: &mut KeyringService,
        analytics: &AnalyticsService,
    ) -> Result<(), String> {
        analytics.clear_connected_server();
        analytics.clear_player();

        let current_server = state.current_server.clone();

        state.clear_api_client();

        if let Some(ref server_url) = current_server {
            if let Err(e) = keyring.delete_credentials(server_url) {
                log::warn!("Failed to clear keyring credentials: {}", e);
            }
        }

        let store = self
            .app_handle
            .store("store.json")
            .map_err(|e| format!("Failed to access store: {}", e))?;

        store.delete("current_server");
        store.delete("current_player");

        if let Some(current_server_url) = current_server {
            if let Some(server_list_value) = store.get("server_list") {
                if let Ok(mut server_list) =
                    serde_json::from_value::<Vec<ServerListEntry>>(server_list_value)
                {
                    server_list.retain(|entry| entry.server != current_server_url);

                    let updated_list = serde_json::to_value(server_list)
                        .map_err(|e| format!("Failed to serialize server list: {}", e))?;
                    store.set("server_list", updated_list);
                }
            }
        }

        store
            .save()
            .map_err(|e| format!("Failed to save store: {}", e))?;

        state.current_server = None;

        Ok(())
    }
}
