//! Core position update logic, usable from both HTTP routes and FFI.
//!
//! This module contains the business logic for handling player position updates,
//! separated from the HTTP transport layer so it can be called directly via FFI.

use common::structs::packet::{
    PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket, QuicNetworkPacketData,
};

use crate::stream::quic::WebhookReceiver;

const PLAYERS_PER_CHUNK: usize = 30;

/// Send player positions to connected QUIC clients.
///
/// This is the core broadcasting logic that can be called from:
/// - HTTP route handler (update_position)
/// - FFI layer (bvc_update_positions)
///
/// # Arguments
/// * `players` - Vector of player position data
/// * `webhook_receiver` - Channel to send packets to QUIC server
pub async fn broadcast_positions(
    players: Vec<common::PlayerEnum>,
    webhook_receiver: &WebhookReceiver,
) {
    // Send players to webhook receiver in chunks
    let mut player_buffer = Vec::with_capacity(PLAYERS_PER_CHUNK);
    for player in players {
        player_buffer.push(player);

        if player_buffer.len() >= PLAYERS_PER_CHUNK {
            send_player_chunk(&player_buffer, webhook_receiver).await;
            player_buffer.clear();
        }
    }

    // Send any remaining players
    if !player_buffer.is_empty() {
        send_player_chunk(&player_buffer, webhook_receiver).await;
    }
}

/// Send a chunk of player data as a QUIC packet.
pub async fn send_player_chunk(players: &[common::PlayerEnum], webhook_receiver: &WebhookReceiver) {
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
