pub mod voice;

pub use voice::VoiceFrame;

use serde::{Deserialize, Serialize};

use crate::errors::PeerWireError;
use crate::structs::packet::MAX_DATAGRAM_SIZE;

// Everything that travels as an unreliable datagram on a peer link.
//
// Append-only, for the same reason as the control frames: postcard encodes a
// variant as its index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Datagram {
    Voice(VoiceFrame),
}

impl Datagram {
    // The size check is on both sides deliberately. An oversized outbound frame is
    // this build's bug and must not reach the wire; an oversized inbound one is a
    // peer's, and is refused before any parsing work is done on its behalf.
    pub fn to_datagram(&self) -> Result<Vec<u8>, PeerWireError> {
        let bytes = postcard::to_stdvec(self).map_err(PeerWireError::Encode)?;
        if bytes.len() > MAX_DATAGRAM_SIZE {
            return Err(PeerWireError::TooLarge {
                size: bytes.len(),
                limit: MAX_DATAGRAM_SIZE,
            });
        }
        Ok(bytes)
    }

    pub fn from_datagram(bytes: &[u8]) -> Result<Self, PeerWireError> {
        if bytes.len() > MAX_DATAGRAM_SIZE {
            return Err(PeerWireError::TooLarge {
                size: bytes.len(),
                limit: MAX_DATAGRAM_SIZE,
            });
        }
        postcard::from_bytes(bytes).map_err(PeerWireError::Decode)
    }
}
