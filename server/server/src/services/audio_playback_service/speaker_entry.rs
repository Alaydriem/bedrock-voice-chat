use std::time::Duration;

use common::PlayerEnum;

/// A live playback's speaker, and how long its track runs.
///
/// The duration rides along because the expiry is derived from the track rather than from a
/// fixed TTL, and moka reads it off the value.
#[derive(Clone)]
pub(crate) struct SpeakerEntry {
    pub(crate) player: PlayerEnum,
    pub(crate) duration: Duration,
}
