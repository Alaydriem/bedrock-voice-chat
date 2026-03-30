use crate::analytics::AnalyticsService;
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub(crate) async fn track_event(
    event: AnalyticsEvent,
    data: Option<AnalyticsEventData>,
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<(), String> {
    analytics.track(event, data);
    Ok(())
}
