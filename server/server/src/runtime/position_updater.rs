use common::structs::packet::{
    PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket, QuicNetworkPacketData,
};

use crate::stream::quic::WebhookReceiver;

const PLAYERS_PER_CHUNK: usize = 30;

pub struct PositionUpdater;

impl PositionUpdater {
    pub async fn broadcast_positions(
        players: Vec<common::PlayerEnum>,
        webhook_receiver: &WebhookReceiver,
    ) {
        let mut player_buffer = Vec::with_capacity(PLAYERS_PER_CHUNK);
        for player in players {
            player_buffer.push(player);

            if player_buffer.len() >= PLAYERS_PER_CHUNK {
                Self::send_player_chunk(&player_buffer, webhook_receiver).await;
                player_buffer.clear();
            }
        }

        if !player_buffer.is_empty() {
            Self::send_player_chunk(&player_buffer, webhook_receiver).await;
        }
    }

    async fn send_player_chunk(players: &[common::PlayerEnum], webhook_receiver: &WebhookReceiver) {
        let packet = QuicNetworkPacket {
            owner: Some(PacketOwner {
                name: String::from("api"),
                client_id: vec![0u8; 0],
            }),
            packet_type: PacketType::PlayerData,
            data: QuicNetworkPacketData::PlayerData(PlayerDataPacket {
                players: players.to_vec(),
            }),
        };

        if let Err(e) = webhook_receiver.send_packet(packet).await {
            tracing::error!("Failed to send packet chunk to QUIC server: {}", e);
        }
    }
}
