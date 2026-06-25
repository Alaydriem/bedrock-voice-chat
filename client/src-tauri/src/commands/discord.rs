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

#[tauri::command]
pub(crate) async fn discord_link(
    service: State<'_, Arc<DiscordLinkService>>,
) -> Result<DiscordLinkStatus, String> {
    service.begin_link().await.map_err(|e| e.to_string())?;
    Ok(service.status().await)
}

#[tauri::command]
pub(crate) async fn discord_resync(
    service: State<'_, Arc<DiscordLinkService>>,
) -> Result<DiscordLinkStatus, String> {
    service.resync().await.map_err(|e| e.to_string())?;
    Ok(service.status().await)
}

#[tauri::command]
pub(crate) async fn discord_complete_link(
    fragment: String,
    service: State<'_, Arc<DiscordLinkService>>,
) -> Result<DiscordLinkStatus, String> {
    service
        .complete_link(&fragment)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn discord_unlink(
    service: State<'_, Arc<DiscordLinkService>>,
) -> Result<DiscordLinkStatus, String> {
    service.unlink().await.map_err(|e| e.to_string())
}
