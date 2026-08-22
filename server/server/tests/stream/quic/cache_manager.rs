use bvc_server_lib::services::AudioPlaybackService;
use bvc_server_lib::stream::quic::{CacheManager, CacheTrait, WebhookReceiver};
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::packet::PacketSender;
use common::{Coordinate, Game, Orientation, PlayerEnum};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::harness::RoutingFixture;

fn player(name: &str) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
        coordinates: Coordinate {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: Some("world-abc".to_string()),
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: None,
        bridged_voice: false,
    })
}

fn playback() -> AudioPlaybackService {
    let (tx, _rx) = mpsc::unbounded_channel();
    AudioPlaybackService::new(
        WebhookReceiver::new(tx),
        ".".to_string(),
        CancellationToken::new(),
        1,
    )
}

#[tokio::test]
async fn a_player_resolves_from_the_position_cache() {
    let cm = CacheManager::new();
    cm.players()
        .set(
            Game::Minecraft.membership_key("Alice").to_string(),
            player("Alice"),
        )
        .await;

    let frame = RoutingFixture::audio_packet(player("Alice"), "minecraft:Alice");

    assert!(cm.resolve_speaker(&frame).await.is_some());
}

// A service name is not a player identity. The injected store owns the lifetime of a playback's
// speaker, and the position cache's presence TTL would lapse part-way through a track.
#[tokio::test]
async fn a_service_resolves_from_the_injected_store() {
    let cm = CacheManager::new();
    let playback = playback();
    playback
        .register_speaker(
            "jukebox-abcd1234".to_string(),
            player("jukebox-abcd1234"),
            Duration::from_secs(600),
        )
        .await;
    cm.set_injected_speakers(&playback);

    let frame = RoutingFixture::audio_packet_from_sender(
        player("jukebox-abcd1234"),
        PacketSender::for_service("jukebox-abcd1234"),
    );

    assert!(cm.resolve_speaker(&frame).await.is_some());
}

// A name nothing registered is not a speaker, and routing must treat that as "no position"
// rather than falling through to the other store and finding somebody else's.
#[tokio::test]
async fn an_unregistered_service_resolves_to_nothing() {
    let cm = CacheManager::new();
    cm.set_injected_speakers(&playback());

    let frame = RoutingFixture::audio_packet_from_sender(
        player("jukebox-never"),
        PacketSender::for_service("jukebox-never"),
    );

    assert!(cm.resolve_speaker(&frame).await.is_none());
}

// A player sender must not resolve out of the injected store, and a service sender must not
// resolve out of the position cache. Keyed the same way, they would find each other's.
#[tokio::test]
async fn the_two_stores_do_not_answer_for_each_other() {
    let cm = CacheManager::new();
    let playback = playback();
    playback
        .register_speaker(
            "minecraft:Alice".to_string(),
            player("Alice"),
            Duration::from_secs(600),
        )
        .await;
    cm.set_injected_speakers(&playback);

    let player_frame = RoutingFixture::audio_packet(player("Alice"), "minecraft:Alice");
    assert!(
        cm.resolve_speaker(&player_frame).await.is_none(),
        "a player must not resolve out of the injected store"
    );

    cm.players()
        .set("jukebox-abcd1234".to_string(), player("jukebox-abcd1234"))
        .await;
    let service_frame = RoutingFixture::audio_packet_from_sender(
        player("jukebox-abcd1234"),
        PacketSender::for_service("jukebox-abcd1234"),
    );
    assert!(
        cm.resolve_speaker(&service_frame).await.is_none(),
        "a service must not resolve out of the position cache"
    );
}
