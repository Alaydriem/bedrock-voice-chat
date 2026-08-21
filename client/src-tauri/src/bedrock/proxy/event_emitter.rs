use std::sync::Arc;

use common::PlayerEnum;
use common::structs::control::ClientAction;
use common::structs::packet::{
    BedrockEvent, BedrockEventDirection, BedrockEventPacket, ClientActionPacket, PacketDirection,
    PacketType, PlayerPositionPacket,
    QuicNetworkPacket, QuicNetworkPacketData,
};
use log::{trace, warn};

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
                data: QuicNetworkPacketData::BedrockEvent(bedrock_packet),
                            // Not a server fan-out, so this envelope carries no sequence.
                ..Default::default()
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

    pub fn try_send_client_action(&self, action: ClientAction) {
        let ca_packet = ClientActionPacket::new(action, PacketDirection::ServerBound);
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::ClientAction,
                data: QuicNetworkPacketData::ClientAction(ca_packet),
                            // Not a server fan-out, so this envelope carries no sequence.
                ..Default::default()
            },
        };

        match self.tx.try_send(packet) {
            Ok(()) => trace!("Bedrock control action queued for QUIC transport"),
            Err(flume::TrySendError::Full(_)) => {
                warn!("Network packet queue full; dropping control action");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                warn!("Network packet channel disconnected; dropping control action");
            }
        }
    }

    pub fn try_send_position(&self, player: PlayerEnum) {
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::PlayerPosition,
                data: QuicNetworkPacketData::PlayerPosition(PlayerPositionPacket { player }),
                            // Not a server fan-out, so this envelope carries no sequence.
                ..Default::default()
            },
        };

        match self.tx.try_send(packet) {
            Ok(()) => trace!("Bedrock position queued for QUIC transport"),
            Err(flume::TrySendError::Full(_)) => {
                curia::warn!("network packet queue full; dropping bedrock position", {
                    defect: crate::logging::Defect::PositionFeedStalled,
                    game: "bedrock",
                });
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                curia::warn!("network packet channel disconnected; dropping bedrock position", {
                    defect: crate::logging::Defect::PositionFeedStalled,
                    game: "bedrock",
                });
            }
        }
    }

}
