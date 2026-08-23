pub mod accept;
pub mod audio_available;
pub mod audio_query;
pub mod hello;
pub mod refuse;
pub mod refuse_reason;

pub use accept::Accept;
pub use audio_available::AudioAvailable;
pub use audio_query::AudioQuery;
pub use hello::Hello;
pub use refuse::Refuse;
pub use refuse_reason::RefuseReason;

use serde::{Deserialize, Serialize};

use crate::errors::PeerWireError;

// Everything that travels on a peer link's control stream.
//
// Postcard encodes a variant as its index, so this list is append-only: a variant
// inserted anywhere but the end shifts every later discriminant and silently
// mis-decodes frames from a peer built against a different order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlFrame {
    Hello(Hello),
    Accept(Accept),
    Refuse(Refuse),
    AudioQuery(AudioQuery),
    AudioAvailable(AudioAvailable),
}

impl ControlFrame {
    pub fn encode(&self) -> Result<Vec<u8>, PeerWireError> {
        postcard::to_stdvec(self).map_err(PeerWireError::Encode)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, PeerWireError> {
        postcard::from_bytes(bytes).map_err(PeerWireError::Decode)
    }
}
