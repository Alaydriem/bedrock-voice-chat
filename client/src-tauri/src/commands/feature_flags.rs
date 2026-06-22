use crate::feature_flags::FeatureFlagService;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub(crate) async fn get_feature_flag(
    flag: String,
    service: State<'_, Arc<FeatureFlagService>>,
) -> Result<bool, String> {
    Ok(service.is_enabled(&flag).await)
}

#[tauri::command]
pub(crate) async fn refresh_feature_flags(
    service: State<'_, Arc<FeatureFlagService>>,
) -> Result<(), String> {
    service.refresh().await
}
