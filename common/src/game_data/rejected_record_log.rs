use std::sync::LazyLock;
use std::time::Duration;

use moka::sync::Cache;

use super::BoundedLogText;

// Errors logged within the window. A mod sends its batch every tick, so one bad record
// would otherwise log the same line many times a second.
static RECENT: LazyLock<Cache<String, ()>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(RejectedRecordLog::CAPACITY)
        .time_to_live(RejectedRecordLog::WINDOW)
        .build()
});

/// Logs a player record that was dropped from a position batch, once per distinct error per
/// window.
pub struct RejectedRecordLog;

impl RejectedRecordLog {
    const CAPACITY: u64 = 256;
    const WINDOW: Duration = Duration::from_secs(60);

    pub fn observe(error: &str) {
        // serde embeds the offending value in its message, which a mod can make as large as
        // the request body.
        let error = BoundedLogText::of(error);
        if RECENT.contains_key(&error) {
            return;
        }
        RECENT.insert(error.clone(), ());

        crate::curia::warn!("dropped a player record that did not parse", {
            "error": error,
        });
    }
}
