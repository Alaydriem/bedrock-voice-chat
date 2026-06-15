use serde::{Deserialize, Serialize};

// Broadcast by a fulfiller over a peer link to ask whether a peer holds a
// given audio file. A peer that has the file answers with `AudioAvailable`.
// The `correlation_id` lets concurrent queries for the same `audio_id` resolve
// independently; the responder echoes it back unchanged in `AudioAvailable`.
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct AudioQuery {
    pub audio_id: String,
    pub correlation_id: String,
}
