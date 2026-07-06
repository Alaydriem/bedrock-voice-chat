use std::sync::Arc;

use common::structs::{AnalyticsEvent, AnalyticsEventData, DiscordLinkStatus};
use tauri::State;

use crate::analytics::AnalyticsService;
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
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<DiscordLinkStatus, String> {
    let was_linked = service.status().await.linked;
    let status = service
        .complete_link(&fragment)
        .await
        .map_err(|e| e.to_string())?;
    let event = if was_linked {
        AnalyticsEvent::DiscordAccountResynced
    } else {
        AnalyticsEvent::DiscordAccountLinked
    };
    analytics.track(
        event,
        Some(AnalyticsEventData::new().insert("role_count", status.role_count as i64)),
    );
    Ok(status)
}

#[tauri::command]
pub(crate) async fn discord_unlink(
    service: State<'_, Arc<DiscordLinkService>>,
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<DiscordLinkStatus, String> {
    let status = service.unlink().await.map_err(|e| e.to_string())?;
    analytics.track(AnalyticsEvent::DiscordAccountUnlinked, None);
    Ok(status)
}
