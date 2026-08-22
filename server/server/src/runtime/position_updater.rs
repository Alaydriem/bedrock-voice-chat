use common::structs::packet::{PacketSender, PlayerDataChunker};

use crate::stream::quic::WebhookReceiver;

pub struct PositionUpdater;

impl PositionUpdater {
    pub async fn broadcast_positions(
        players: Vec<common::PlayerEnum>,
        webhook_receiver: &WebhookReceiver,
    ) {
        let sender = PacketSender::for_service(PacketSender::SERVER_API);

        for chunk in PlayerDataChunker::chunk(players, Some(&sender)) {
            Self::send_player_chunk(chunk, &sender, webhook_receiver).await;
        }
    }

    async fn send_player_chunk(
        players: Vec<common::PlayerEnum>,
        sender: &PacketSender,
        webhook_receiver: &WebhookReceiver,
    ) {
        let packet = PlayerDataChunker::packet(players, Some(sender));

        if let Err(e) = webhook_receiver.send_packet(packet).await {
            tracing::error!("Failed to send packet chunk to QUIC server: {}", e);
        }
    }
}
