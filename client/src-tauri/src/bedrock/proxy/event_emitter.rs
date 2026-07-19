use std::sync::Arc;

use common::PlayerEnum;
use common::structs::control::ClientAction;
use common::structs::packet::{
    BedrockEvent, BedrockEventDirection, BedrockEventPacket, ClientActionPacket, PacketDirection,
    PacketType, PeerAnnounceObservedPacket, PeerPresenceObservedPacket, PlayerPositionPacket,
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

    pub fn try_send_client_action(&self, action: ClientAction) {
        let ca_packet = ClientActionPacket::new(action, PacketDirection::ServerBound);
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::ClientAction,
                owner: None,
                data: QuicNetworkPacketData::ClientAction(ca_packet),
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

    pub fn try_send_observed(&self, token: String) {
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::PeerPresenceObserved,
                owner: None,
                data: QuicNetworkPacketData::PeerPresenceObserved(PeerPresenceObservedPacket {
                    token,
                }),
            },
        };

        match self.tx.try_send(packet) {
            Ok(()) => trace!("Bedrock presence observation queued for QUIC transport"),
            Err(flume::TrySendError::Full(_)) => {
                warn!("Network packet queue full; dropping bedrock presence observation");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                warn!("Network packet channel disconnected; dropping bedrock presence observation");
            }
        }
    }

    pub fn try_send_announce_observed(&self, hashed_world: String, endpoint: String) {
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type: PacketType::PeerAnnounceObserved,
                owner: None,
                data: QuicNetworkPacketData::PeerAnnounceObserved(PeerAnnounceObservedPacket {
                    hashed_world,
                    endpoint,
                }),
            },
        };

        match self.tx.try_send(packet) {
            Ok(()) => trace!("Bedrock announce observation queued for QUIC transport"),
            Err(flume::TrySendError::Full(_)) => {
                warn!("Network packet queue full; dropping bedrock announce observation");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                warn!("Network packet channel disconnected; dropping bedrock announce observation");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_send_observed_emits_observed_packet() {
        let (tx, rx) = flume::unbounded::<NetworkPacket>();
        let emitter = BedrockEventEmitter::new(Arc::new(tx));

        emitter.try_send_observed("tok".to_string());

        let packet = rx.try_recv().expect("observed packet should be queued");
        assert_eq!(packet.data.packet_type, PacketType::PeerPresenceObserved);
        match packet.data.data {
            QuicNetworkPacketData::PeerPresenceObserved(observed) => {
                assert_eq!(observed.token, "tok");
            }
            other => panic!("expected PeerPresenceObserved, got {:?}", other),
        }
    }
}
