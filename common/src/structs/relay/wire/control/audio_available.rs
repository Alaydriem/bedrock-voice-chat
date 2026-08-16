use serde::{Deserialize, Serialize};

// A peer's answer that it holds the queried clip and will serve it on a stream.
//
// Carries no token and no endpoint: the transfer rides the connection this frame
// arrived on, which is already authenticated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioAvailable {
    pub correlation_id: String,
    pub audio_id: String,
}
