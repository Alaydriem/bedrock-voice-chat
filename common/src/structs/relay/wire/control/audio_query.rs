use serde::{Deserialize, Serialize};

// Asks whether a peer holds a clip.
//
// `correlation_id` rather than `audio_id` identifies the exchange, so two
// concurrent plays of the same file resolve independently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioQuery {
    pub audio_id: String,
    pub correlation_id: String,
}
