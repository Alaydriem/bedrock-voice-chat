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
    pub seq: Option<EnvelopeSequence>,
    pub packet_type: PacketType,
    pub data: QuicNetworkPacketData,
    pub sender: Option<PacketSender>,
}

impl Default for QuicNetworkPacket {
    fn default() -> Self {
        Self {
            packet_type: PacketType::Debug,
            data: QuicNetworkPacketData::Debug(super::debug_packet::DebugPacket {
                version: String::new(),
                timestamp: 0,
            }),
            seq: None,
            sender: None,
        }
    }
}

impl QuicNetworkPacket {
    pub const SEQ_TAG_OFFSET: usize = 0;
    pub const SEQ_VALUE_RANGE: std::ops::Range<usize> = 1..5;

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

    /// The authenticated player this came from, if the packet names one.
    ///
    /// Absent on anything a client built, which is every packet on the inbound path before
    /// `PacketIdentityStamp` runs. Also absent on a reduced audio frame, where the identity
    /// was elided and the receiver resolves it from `sender_device`.
    pub fn sender_identity(&self) -> Option<&crate::PlayerIdentity> {
        self.sender.as_ref().and_then(|s| s.identity())
    }

    /// The connection this came from, absent for a relayed player and for anything the
    /// server injected.
    pub fn sender_device(&self) -> Option<u64> {
        self.sender.as_ref().and_then(|s| s.device())
    }

    /// The key this packet's sender is routed and keyed on, whether a player or a service.
    ///
    /// Audio routing needs one key for both: jukebox and webhook audio hold channel
    /// membership and a cached position under their service name exactly as a player holds
    /// them under their identity.
    pub fn sender_key(&self) -> Option<String> {
        self.sender.as_ref().and_then(|s| s.routing_key())
    }

    /// The server surface that injected this, when a service did.
    pub fn sender_service(&self) -> Option<&str> {
        self.sender.as_ref().and_then(|s| s.service())
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
                                let Some(identity) = self.sender_identity() else {
                                    return;
                                };
                                if let Some(sender_player) =
                                    player_data.get(&identity.to_string()).await
                                {
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
