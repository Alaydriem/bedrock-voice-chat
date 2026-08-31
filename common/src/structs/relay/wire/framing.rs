use crate::errors::PeerWireError;

// Length-prefixed framing for the control stream.
//
// A postcard frame is not self-delimiting and the control stream is a byte
// stream, so the length has to be stated. Four bytes big-endian rather than a
// varint: the header is read before anything is known about the sender, and a
// fixed width means the read is a single fixed-size call that cannot itself be
// made to block on a malformed length.
pub struct Framing;

impl Framing {
    // Far above any real control frame, and far below anything worth allocating
    // for a peer that has not yet been authorized.
    pub const MAX_FRAME: usize = 64 * 1024;

    pub const HEADER_LEN: usize = 4;

    // Generic over the frame type because two wires share this framing: the peer
    // control stream and the enrollment stream. They version and evolve
    // independently, so they must not share a frame enum — but the length prefix
    // that delimits them is the same problem in both.
    pub fn encode<T: serde::Serialize>(frame: &T) -> Result<Vec<u8>, PeerWireError> {
        let payload = postcard::to_stdvec(frame).map_err(PeerWireError::Encode)?;

        if payload.len() > Self::MAX_FRAME {
            return Err(PeerWireError::TooLarge {
                size: payload.len(),
                limit: Self::MAX_FRAME,
            });
        }

        let mut out = Vec::with_capacity(Self::HEADER_LEN + payload.len());
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&payload);
        Ok(out)
    }

    pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, PeerWireError> {
        postcard::from_bytes(bytes).map_err(PeerWireError::Decode)
    }

    // A zero length is refused alongside an oversized one: there is no empty
    // control frame, so zero means a confused or hostile sender either way.
    pub fn payload_len(header: &[u8; Self::HEADER_LEN]) -> Result<usize, PeerWireError> {
        let len = u32::from_be_bytes(*header) as usize;

        if len == 0 || len > Self::MAX_FRAME {
            return Err(PeerWireError::TooLarge {
                size: len,
                limit: Self::MAX_FRAME,
            });
        }

        Ok(len)
    }
}
