use anyhow::{Error, anyhow};
use serde::{Deserialize, Serialize};

use moka::future::Cache;
use std::sync::Arc;

use super::audio_frame_packet::AudioFramePacket;
use super::envelope_sequence::EnvelopeSequence;
use super::packet_sender::PacketSender;
use super::packet_type::PacketType;
use super::quic_network_packet_data::QuicNetworkPacketData;

pub const MAX_DATAGRAM_SIZE: usize = 1150;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuicNetworkPacket {
    // Monotonic per-connection sequence, assigned by the sender at the moment it queues this
    // datagram for one recipient. A gap at the receiver therefore means exactly one thing: the
    // sender sent it and it did not arrive.
    //
    // First field, and fixed-width, so its encoded bytes occupy a constant range at the head of
    // every datagram whatever the packet carries. The audio fan-out serializes one envelope per
    // frame and rewrites `SEQ_VALUE_RANGE` for each recipient rather than re-encoding per
    // recipient, which is only sound while that range does not move.
    //
    // On the envelope rather than on the audio frame deliberately. A per-speaker sequence would be
    // gapped by the server's own routing — proximity, channel membership, deafen distance — and a
    // gap would then conflate correct filtering with loss.
    //
    // `None` from every producer that is not a server fan-out to one connection — the relay dialer,
    // the embedded FFI path, the mods, and the client itself. A receiver reports loss as unmeasured
    // rather than zero when it is absent, so an unstamped peer never reads as a perfect link.
    //
    // Postcard is not self-describing and its format is positional, so the shape of this struct is
    // a BREAKING wire change in both directions and `#[serde(default)]` is deliberately absent
    // because it would imply otherwise.
    // `common/tests/structs/packet/envelope_sequence.rs` pins both directions and this layout.
    pub seq: Option<EnvelopeSequence>,
    pub packet_type: PacketType,
    pub data: QuicNetworkPacketData,
    /// Who sent this, as the server determined it from the mTLS certificate.
    ///
    /// Present on everything the server fans out and absent on everything a client sends,
    /// because a client has nothing to say about its own identity.
    ///
    /// Last field deliberately. Postcard is positional, so an `Option` between `packet_type` and
    /// `data` would be read as the start of `data` by any decoder that does not expect it.
    pub sender: Option<PacketSender>,
}

// Exists so producers can write `..Default::default()` and leave `seq` alone, rather than
// repeating `seq: None` at every construction site.
//
// Written out rather than derived on purpose. Deriving would require `Default` on `PacketType` and
// `QuicNetworkPacketData`, which would make `PacketType::default()` callable and let a missing
// assignment put a silently mis-tagged datagram on the wire. Every real construction overrides both
// fields, so the values chosen here are never transmitted.
impl Default for QuicNetworkPacket {
    fn default() -> Self {
        Self {
            packet_type: PacketType::Debug,
            data: QuicNetworkPacketData::Debug(super::debug_packet::DebugPacket {
                identity: String::new(),
                version: String::new(),
                timestamp: 0,
            }),
            seq: None,
            sender: None,
        }
    }
}

impl QuicNetworkPacket {
    // Byte layout of the encoded `seq` field. Postcard is positional and this is the first field,
    // so the `Option` tag sits at index 0 and the fixed-width value in 1..5. The fan-out rewrites
    // exactly this range on an already-serialized envelope; `sequence_bytes_sit_at_a_fixed_offset`
    // pins it.
    pub const SEQ_TAG_OFFSET: usize = 0;
    pub const SEQ_VALUE_RANGE: std::ops::Range<usize> = 1..5;

    // Stamps this envelope for one recipient. Called immediately before serialization, after every
    // decision not to send, so a suppressed packet consumes no number.
    pub fn stamp(&mut self, sequence: u32) {
        self.seq = Some(EnvelopeSequence(sequence));
    }

    pub fn sequence(&self) -> Option<u32> {
        self.seq.map(|s| s.0)
    }

    pub fn to_datagram(&self) -> Result<Vec<u8>, anyhow::Error> {
        let bytes = postcard::to_stdvec(&self)?;
        if bytes.len() > MAX_DATAGRAM_SIZE {
            return Err(anyhow!(
                "Serialized datagram size {} exceeds max {}",
                bytes.len(),
                MAX_DATAGRAM_SIZE
            ));
        }
        Ok(bytes)
    }

    pub fn from_datagram(data: &[u8]) -> Result<Self, anyhow::Error> {
        if data.len() > MAX_DATAGRAM_SIZE {
            return Err(anyhow!(
                "Incoming datagram size {} exceeds max {}",
                data.len(),
                MAX_DATAGRAM_SIZE
            ));
        }
        postcard::from_bytes::<QuicNetworkPacket>(data)
            .map_err(|e| anyhow!("Postcard deserialization error: {}", e))
    }

    pub fn get_packet_type(&self) -> PacketType {
        self.packet_type.clone()
    }

    /// The authenticated identity of whoever sent this, if the server has stamped it.
    ///
    /// Absent on anything a client built, which is every packet on the inbound path before
    /// `PacketIdentityStamp` runs. A reader that needs an identity there is reading too early.
    pub fn sender_identity(&self) -> Option<&str> {
        self.sender.as_ref().map(|s| s.identity.as_str())
    }

    /// The connection this came from, absent for a synthetic sender the server injected.
    pub fn sender_device(&self) -> Option<u64> {
        self.sender.as_ref().and_then(|s| s.device)
    }

    pub fn get_data(&self) -> Option<&QuicNetworkPacketData> {
        Some(&self.data)
    }

    pub async fn update_coordinates(&mut self, player_data: Arc<Cache<String, crate::PlayerEnum>>) {
        match self.get_packet_type() {
            PacketType::AudioFrame => match self.get_data() {
                Some(data) => {
                    let data = data.to_owned();
                    let data: Result<AudioFramePacket, ()> = data.try_into();

                    match data {
                        Ok(mut data) => {
                            if data.sender.is_none() {
                                // Keyed on the stamped identity, which is already canonical, so
                                // this hits the same key the position ingress writes.
                                let identity = self.sender_identity().unwrap_or_default().to_string();
                                if let Some(sender_player) = player_data.get(&identity).await {
                                    data.sender = Some(sender_player);
                                    let audio_frame: QuicNetworkPacketData =
                                        QuicNetworkPacketData::AudioFrame(data);
                                    self.data = audio_frame;
                                }
                            }
                        }
                        Err(_) => {
                            tracing::error!("Could not downcast reference packet to audio frame");
                        }
                    }
                }
                None => {
                    tracing::error!("Could not downcast reference packet to audio frame");
                }
            },
            _ => {}
        }
    }

    pub fn to_string(&self) -> Result<String, Error> {
        match ron::to_string(&self) {
            Ok(message) => Ok(message),
            Err(e) => {
                tracing::error!(
                    "Could not convert QuicNetworkPacket back to String {}",
                    e.to_string()
                );
                Err(anyhow!(e.to_string()))
            }
        }
    }

    pub fn from_string(message: String) -> Option<QuicNetworkPacket> {
        return match ron::from_str::<QuicNetworkPacket>(&String::from_utf8_lossy(
            message.as_bytes(),
        )) {
            Ok(packet) => Some(packet),
            Err(e) => {
                tracing::error!(
                    "Could not decode QuicNetworkPacket from string {}",
                    e.to_string()
                );
                None
            }
        };
    }
}
