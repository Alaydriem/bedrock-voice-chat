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
            bridged_voice: false,
        })
    }

    // A canonical name is a player; anything else is a service, which is how the fixture
    // expresses server-injected audio such as jukebox playback.
    fn sender_for(identity: &str) -> PacketSender {
        match identity.parse::<common::PlayerIdentity>() {
            Ok(identity) => PacketSender::player(identity, 1),
            Err(_) => PacketSender::for_service(identity),
        }
    }

    pub fn audio_packet(sender: PlayerEnum, sender_identity: &str) -> QuicNetworkPacket {
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            sender: Some(Self::sender_for(sender_identity)),
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![0u8; 160],
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
            sender: Some(Self::sender_for(sender_identity)),
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![0u8; 160],
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
            cache.insert(p.identity().to_string(), p.clone()).await;
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

    // The whole delivered envelope, for asserting on the sender rather than the frame.
    // None when nothing was delivered within the timeout.
    pub async fn delivered_envelope(
        rx: &mut mpsc::Receiver<RoutedPacket>,
    ) -> Option<QuicNetworkPacket> {
        match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Some(RoutedPacket::Serialized(bytes))) => {
                Some(QuicNetworkPacket::from_datagram(&bytes).expect("routed datagram decodes"))
            }
            _ => None,
        }
    }

    // The full decoded audio frame a recipient received, for asserting fields beyond
    // the spatial flag. None when nothing was delivered within the timeout.
    pub async fn delivered_frame(rx: &mut mpsc::Receiver<RoutedPacket>) -> Option<AudioFramePacket> {
        match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Some(RoutedPacket::Serialized(bytes))) => {
                let packet =
                    QuicNetworkPacket::from_datagram(&bytes).expect("routed datagram decodes");
                match packet.data {
                    QuicNetworkPacketData::AudioFrame(af) => Some(af),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
