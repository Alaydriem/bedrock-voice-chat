use common::structs::packet::MAX_DATAGRAM_SIZE;
use common::structs::relay::wire::Datagram;
use common::structs::relay::wire::datagram::VoiceFrame;
use iroh::PublicKey;
use iroh::endpoint::Connection;

use super::error::PeerError;

// One live peer connection.
//
// Voice rides datagrams rather than a stream: a late frame is worthless, so
// head-of-line blocking would trade something valuable for something that is not.
//
// `Clone` because an iroh `Connection` is a cheap handle: a receive pump owns one
// and a link table another, and both refer to the same connection.
#[derive(Clone)]
pub struct PeerLink {
    conn: Connection,
    node: PublicKey,
    worlds: Vec<String>,
}

impl PeerLink {
    // The cap is asserted here rather than lowered on a guess.
    //
    // Measurement put iroh's usable datagram at 1162 bytes on both a direct and a
    // relayed path, against BVC's 1150 — twelve bytes of margin, and no evidence
    // for any particular smaller number. A link that cannot carry a full frame is
    // refused at establishment, which turns what would otherwise be silent
    // mid-call frame loss into one loud failure at connect.
    pub fn establish(conn: Connection, worlds: Vec<String>) -> Result<Self, PeerError> {
        let negotiated = conn.max_datagram_size().unwrap_or(0);
        if negotiated < MAX_DATAGRAM_SIZE {
            return Err(PeerError::DatagramTooSmall {
                negotiated,
                required: MAX_DATAGRAM_SIZE,
            });
        }

        let node = conn.remote_id();
        Ok(Self { conn, node, worlds })
    }

    pub fn node(&self) -> PublicKey {
        self.node
    }

    pub fn worlds(&self) -> &[String] {
        &self.worlds
    }

    pub fn carries_world(&self, world: &str) -> bool {
        self.worlds.iter().any(|w| w == world)
    }

    pub fn send(&self, frame: VoiceFrame) -> Result<(), PeerError> {
        let bytes = Datagram::Voice(frame).to_datagram()?;
        self.conn
            .send_datagram(bytes.into())
            .map_err(|e| PeerError::Transport(e.to_string()))
    }

    pub async fn recv(&self) -> Result<VoiceFrame, PeerError> {
        let bytes = self
            .conn
            .read_datagram()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        match Datagram::from_datagram(&bytes)? {
            Datagram::Voice(frame) => Ok(frame),
        }
    }
}
