use std::sync::Arc;
use std::time::Duration;

use bvc_server_lib::stream::quic::connection::RoutedPacket;
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::packet::{
    AudioFramePacket, PacketSender, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::{Coordinate, Orientation, PlayerEnum};
use moka::future::Cache;
use tokio::sync::mpsc;

/// Builders and observers for audio-routing tests: positioned players, the
/// audio packets they emit, the player cache the router reads, and the routed
/// frame's spatial flag as seen by a recipient's channel.
pub struct RoutingFixture;

impl RoutingFixture {
    pub fn player(name: &str, x: f32, deafen: bool) -> PlayerEnum {
        PlayerEnum::Minecraft(MinecraftPlayer {
            name: name.to_string(),
            coordinates: Coordinate { x, y: 64.0, z: 0.0 },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: None,
        })
    }

    pub fn audio_packet(sender: PlayerEnum, sender_identity: &str) -> QuicNetworkPacket {
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            sender: Some(PacketSender::new(sender_identity.to_string(), 1)),
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![0u8; 160],
                48000,
                Some(sender),
                Some(true),
            )),
            // Not a server fan-out to one connection, so this envelope carries no sequence.
            ..Default::default()
        }
    }

    // An audio frame from a sender that carries NO PlayerEnum: what a client emits
    // before it has any position, i.e. it joined a channel but not the game yet.
    pub fn audio_packet_without_position(sender_identity: &str) -> QuicNetworkPacket {
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            sender: Some(PacketSender::new(sender_identity.to_string(), 1)),
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![0u8; 160],
                48000,
                None,
                Some(true),
            )),
            // Not a server fan-out to one connection, so this envelope carries no sequence.
            ..Default::default()
        }
    }

    pub async fn player_cache(players: &[PlayerEnum]) -> Arc<Cache<String, PlayerEnum>> {
        let cache: Arc<Cache<String, PlayerEnum>> = Arc::new(Cache::builder().build());
        for p in players {
            use common::traits::player_data::PlayerData;
            // Keyed the way the router reads it: on the canonical identity, not the
            // bare name. Seeding it bare made every lookup miss.
            cache.insert(p.identity(), p.clone()).await;
        }
        cache
    }

    // The delivered frame's spatial flag, or None when nothing arrives within
    // the timeout (i.e. the router filtered this recipient out).
    pub async fn delivered_spatial(rx: &mut mpsc::Receiver<RoutedPacket>) -> Option<bool> {
        match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Some(RoutedPacket::Serialized(bytes))) => {
                let packet =
                    QuicNetworkPacket::from_datagram(&bytes).expect("routed datagram decodes");
                match packet.data {
                    QuicNetworkPacketData::AudioFrame(af) => af.spatial,
                    _ => panic!("expected an AudioFrame datagram"),
                }
            }
            _ => None,
        }
    }
}
