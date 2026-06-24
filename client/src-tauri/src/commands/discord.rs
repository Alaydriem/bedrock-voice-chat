use std::sync::Arc;

use common::structs::DiscordLinkStatus;
use tauri::State;

use crate::discord::DiscordLinkService;

#[tauri::command]
pub(crate) async fn discord_status(
    service: State<'_, Arc<DiscordLinkService>>,
) -> Result<DiscordLinkStatus, String> {
    Ok(service.status().await)
}

#[cfg(desktop)]
#[tauri::command]
pub(crate) async fn discord_link(
    service: State<'_, Arc<DiscordLinkService>>,
) -> Result<DiscordLinkStatus, String> {
    service.link().await.map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
pub(crate) async fn discord_resync(
    service: State<'_, Arc<DiscordLinkService>>,
) -> Result<DiscordLinkStatus, String> {
    service.resync().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn discord_unlink(
    service: State<'_, Arc<DiscordLinkService>>,
) -> Result<DiscordLinkStatus, String> {
    service.unlink().await.map_err(|e| e.to_string())
}
