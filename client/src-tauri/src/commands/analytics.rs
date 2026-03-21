use std::sync::Arc;
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use tauri::State;
use crate::analytics::AnalyticsService;

#[tauri::command]
pub(crate) async fn track_event(
    event: AnalyticsEvent,
    data: Option<AnalyticsEventData>,
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<(), String> {
    analytics.track(event, data);
    Ok(())
}
