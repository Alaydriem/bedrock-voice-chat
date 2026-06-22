use serde::{Deserialize, Serialize};

// Sent back over a peer link in reply to an `AudioQuery` by a peer that holds
// the requested file. The `stream_token` is what the fulfiller uses to later
// HTTP-pull the `.opus` from the responding peer. The `correlation_id` echoes
// the one from the originating `AudioQuery` so the fulfiller pairs the reply
// with the exact outstanding query, even when several share an `audio_id`.
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct AudioAvailable {
    pub audio_id: String,
    pub stream_token: String,
    pub correlation_id: String,
}
