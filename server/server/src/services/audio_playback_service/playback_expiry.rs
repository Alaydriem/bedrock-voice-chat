use std::time::Duration;

use moka::Expiry;

use super::playback_entry::PlaybackEntry;

/// Per-entry expiry based on the playback duration + a safety buffer.
pub(crate) struct PlaybackExpiry;

impl Expiry<String, PlaybackEntry> for PlaybackExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &PlaybackEntry,
        _current_time: std::time::Instant,
    ) -> Option<Duration> {
        // Expire after the playback duration + 5s safety buffer
        Some(value.duration + Duration::from_secs(5))
    }
}
