use std::sync::Arc;

use common::PlayerEnum;
use common::structs::packet::{
    BedrockEvent, BedrockEventPacket, PacketType, PlayerDataPacket, QuicNetworkPacket,
    QuicNetworkPacketData,
};
use log::{debug, warn};

use crate::NetworkPacket;

pub struct BedrockEventEmitter {
    tx: Arc<flume::Sender<NetworkPacket>>,
}

impl BedrockEventEmitter {
    pub fn new(tx: Arc<flume::Sender<NetworkPacket>>) -> Self {
        Self { tx }
    }

    pub fn try_send(&self, event: BedrockEvent, world_uuid: String) {
        let bedrock_packet = BedrockEventPacket::new(event, world_uuid);
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::BedrockEvent,
                owner: None,
                data: QuicNetworkPacketData::BedrockEvent(bedrock_packet),
            },
        };

        match self.tx.try_send(packet) {
            Ok(()) => debug!("Bedrock event queued for QUIC transport"),
            Err(flume::TrySendError::Full(_)) => {
                warn!("Network packet queue full; dropping bedrock event");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                warn!("Network packet channel disconnected; dropping bedrock event");
            }
        }
    }

    pub fn try_send_player_data(&self, player: PlayerEnum) {
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::PlayerData,
                owner: None,
                data: QuicNetworkPacketData::PlayerData(PlayerDataPacket {
                    players: vec![player],
                }),
            },
        };

        match self.tx.try_send(packet) {
            Ok(()) => debug!("Bedrock position heartbeat queued for QUIC transport"),
            Err(flume::TrySendError::Full(_)) => {
                warn!("Network packet queue full; dropping bedrock position heartbeat");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                warn!("Network packet channel disconnected; dropping bedrock position heartbeat");
            }
        }
    }
}
