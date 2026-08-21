use tauri_plugin_curia::curia::LogEvent;

pub struct QueuedEvent {
    pub event: LogEvent,
    pub suppressed: u32,
    // Every queued event becomes a breadcrumb. Only a warning or worse also
    // becomes a Sentry Log; quiet traffic rides along as trail for whatever
    // Issue is captured next.
    pub as_log: bool,
}
