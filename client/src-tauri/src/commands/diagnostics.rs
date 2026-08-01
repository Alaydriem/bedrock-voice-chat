use std::sync::Arc;

use common::structs::metrics::LinkDiagnosticsSnapshot;
use tauri::State;

use crate::diagnostics::LinkDiagnosticsService;

// The initial render, before the first push arrives. Absent while disconnected rather than a
// snapshot of zeros, which a panel would draw as a flawless link.
#[tauri::command]
pub(crate) async fn get_link_diagnostics(
    service: State<'_, Arc<LinkDiagnosticsService>>,
) -> Result<Option<LinkDiagnosticsSnapshot>, String> {
    Ok(service.snapshot())
}

// Backs both the status panel's copy action and the diagnostics settings page, so the two cannot
// hand a player different text.
#[tauri::command]
pub(crate) async fn get_diagnostics_report(
    service: State<'_, Arc<LinkDiagnosticsService>>,
) -> Result<String, String> {
    Ok(service.render_report())
}
