use std::time::Duration;

use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct PlaybackEntry {
    pub(crate) cancel_token: CancellationToken,
    pub(crate) audio_file_id: String,
    pub(crate) duration: Duration,
    // The name this playback's envelopes carry. Held so the speaker can be forgotten from a
    // path that has only the event id, without recomputing the truncation that derives one
    // from the other.
    pub(crate) jukebox_name: String,
}
