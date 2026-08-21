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

// Restarts every measurement from now, without touching the connection.
//
// Almost everything on the status panel counts from the start of the session: drops, concealment
// and the loss ratios are all cumulative, so a rough first minute stays in the numbers for as long
// as the session lasts and "is it bad right now" cannot be read off the panel at all. This is how
// a change gets measured against the thing it changed.
#[tauri::command]
pub(crate) async fn reset_link_diagnostics(
    service: State<'_, Arc<LinkDiagnosticsService>>,
) -> Result<(), String> {
    service.reset_stats();
    Ok(())
}

// Dev-only. The release build has no caller and no button, but the guard is here
// too so a stray invoke cannot pump synthetic data into production Sentry.
#[tauri::command]
pub(crate) async fn logging_smoke_test() -> Result<(), String> {
    use common::consts::variant::Variant;

    if Variant::get() != Variant::Dev {
        return Err("logging smoke test is dev-only".to_string());
    }

    crate::logging::LoggingSmokeTest::run();
    Ok(())
}
