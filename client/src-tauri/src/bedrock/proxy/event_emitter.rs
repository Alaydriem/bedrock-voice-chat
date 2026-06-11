use std::sync::Arc;

use common::PlayerEnum;
use common::structs::packet::{
    BedrockEvent, BedrockEventDirection, BedrockEventPacket, PacketType, PlayerPositionPacket,
    QuicNetworkPacket, QuicNetworkPacketData,
};
use log::{debug, warn, trace};

use crate::NetworkPacket;

pub struct BedrockEventEmitter {
    tx: Arc<flume::Sender<NetworkPacket>>,
}

impl BedrockEventEmitter {
    pub fn new(tx: Arc<flume::Sender<NetworkPacket>>) -> Self {
        Self { tx }
    }

    pub fn try_send(&self, event: BedrockEvent, world_uuid: String) {
        let bedrock_packet = BedrockEventPacket::with_direction(
            event,
            world_uuid,
            BedrockEventDirection::ServerBound,
        );
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::BedrockEvent,
                owner: None,
                data: QuicNetworkPacketData::BedrockEvent(bedrock_packet),
            },
        };

        match self.tx.try_send(packet) {
            Ok(()) => trace!("Bedrock event queued for QUIC transport"),
            Err(flume::TrySendError::Full(_)) => {
                warn!("Network packet queue full; dropping bedrock event");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                warn!("Network packet channel disconnected; dropping bedrock event");
            }
        }
    }

    pub fn try_send_position(&self, player: PlayerEnum) {
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::PlayerPosition,
                owner: None,
                data: QuicNetworkPacketData::PlayerPosition(PlayerPositionPacket { player }),
            },
        };

        match self.tx.try_send(packet) {
            Ok(()) => trace!("Bedrock position queued for QUIC transport"),
            Err(flume::TrySendError::Full(_)) => {
                warn!("Network packet queue full; dropping bedrock position");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                warn!("Network packet channel disconnected; dropping bedrock position");
            }
        }
    }
}
