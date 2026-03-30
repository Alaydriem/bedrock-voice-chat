use std::time::Duration;

use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct PlaybackEntry {
    pub(crate) cancel_token: CancellationToken,
    pub(crate) audio_file_id: String,
    pub(crate) duration: Duration,
}
