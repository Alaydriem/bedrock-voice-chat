use common::structs::packet::MAX_DATAGRAM_SIZE;
use common::structs::relay::wire::datagram::{GroupScope, VoiceFrame};
use common::structs::relay::wire::{Datagram, WireVersion};
use iroh::PublicKey;
use iroh::endpoint::Connection;

use super::error::PeerError;
use super::peer_voice::PeerVoice;
use super::voice_encoding::VoiceEncoding;

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
    version: WireVersion,
}

impl PeerLink {
    // The cap is asserted here rather than lowered on a guess.
    //
    // Measurement put iroh's usable datagram at 1162 bytes on both a direct and a
    // relayed path, against BVC's 1150 — twelve bytes of margin, and no evidence
    // for any particular smaller number. A link that cannot carry a full frame is
    // refused at establishment, which turns what would otherwise be silent
    // mid-call frame loss into one loud failure at connect.
    pub fn establish(
        conn: Connection,
        worlds: Vec<String>,
        version: WireVersion,
    ) -> Result<Self, PeerError> {
        let negotiated = conn.max_datagram_size().unwrap_or(0);
        if negotiated < MAX_DATAGRAM_SIZE {
            return Err(PeerError::DatagramTooSmall {
                negotiated,
                required: MAX_DATAGRAM_SIZE,
            });
        }

        let node = conn.remote_id();
        Ok(Self {
            conn,
            node,
            worlds,
            version,
        })
    }

    pub fn node(&self) -> PublicKey {
        self.node
    }

    pub fn worlds(&self) -> &[String] {
        &self.worlds
    }

    pub fn version(&self) -> WireVersion {
        self.version
    }

    pub fn carries_world(&self, world: &str) -> bool {
        self.worlds.iter().any(|w| w == world)
    }

    // `Ok(false)` when this link's version cannot carry the frame safely, so it was not sent.
    pub fn send(&self, frame: VoiceFrame, group: Option<GroupScope>) -> Result<bool, PeerError> {
        let Some(datagram) = VoiceEncoding::encode(self.version, frame, group) else {
            return Ok(false);
        };
        let bytes = datagram.to_datagram()?;
        self.conn
            .send_datagram(bytes.into())
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        Ok(true)
    }

    pub async fn recv(&self) -> Result<PeerVoice, PeerError> {
        let bytes = self
            .conn
            .read_datagram()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        Ok(match Datagram::from_datagram(&bytes)? {
            Datagram::Voice(frame) => PeerVoice { frame, group: None },
            Datagram::GroupVoice { frame, group } => PeerVoice {
                frame,
                group: Some(group),
            },
        })
    }
}
