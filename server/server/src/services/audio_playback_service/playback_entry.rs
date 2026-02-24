use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

/// Cache entry for an active playback session.
#[derive(Clone)]
pub(crate) struct PlaybackEntry {
    pub(crate) cancel_token: Arc<CancellationToken>,
    pub(crate) audio_file_id: String,
    pub(crate) duration: Duration,
}
