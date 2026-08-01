use common::structs::packet::{PacketOwner, PlayerDataChunker};

use crate::stream::quic::WebhookReceiver;

pub struct PositionUpdater;

impl PositionUpdater {
    pub async fn broadcast_positions(
        players: Vec<common::PlayerEnum>,
        webhook_receiver: &WebhookReceiver,
    ) {
        let owner = Self::owner();

        for chunk in PlayerDataChunker::chunk(players, Some(&owner)) {
            Self::send_player_chunk(chunk, &owner, webhook_receiver).await;
        }
    }

    fn owner() -> PacketOwner {
        PacketOwner {
            name: String::from("api"),
            client_id: vec![0u8; 0],
        }
    }

    async fn send_player_chunk(
        players: Vec<common::PlayerEnum>,
        owner: &PacketOwner,
        webhook_receiver: &WebhookReceiver,
    ) {
        let packet = PlayerDataChunker::packet(players, Some(owner));

        if let Err(e) = webhook_receiver.send_packet(packet).await {
            tracing::error!("Failed to send packet chunk to QUIC server: {}", e);
        }
    }
}
