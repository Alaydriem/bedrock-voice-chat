use std::time::Duration;

use moka::Expiry;

use super::speaker_entry::SpeakerEntry;

/// Expires a jukebox speaker with the track it belongs to.
///
/// Mirrors `PlaybackExpiry`. A fixed TTL here is what would make a long track go silent
/// part-way through and need a refresh timer to survive.
pub(crate) struct SpeakerExpiry;

impl Expiry<String, SpeakerEntry> for SpeakerExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &SpeakerEntry,
        _current_time: std::time::Instant,
    ) -> Option<Duration> {
        Some(value.duration + Duration::from_secs(5))
    }
}
