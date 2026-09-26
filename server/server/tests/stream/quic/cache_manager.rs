use bvc_server_lib::services::AudioPlaybackService;
use bvc_server_lib::stream::quic::connection::ConnectionRegistry;
use bvc_server_lib::stream::quic::{CacheManager, CacheTrait, WebhookReceiver};
use common::consts::audio::WHISPER_CONTROL_TARGET;
use common::structs::control::{PreferenceKey, WhisperPreference};
use common::traits::player_data::PlayerData;
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
        whispering: false,
        spectator: false,
        world_uuid: Some("world-abc".to_string()),
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: None,
        bridged_voice: false,
    })
}

fn whisperer(name: &str) -> PlayerEnum {
    match player(name) {
        PlayerEnum::Minecraft(p) => PlayerEnum::Minecraft(MinecraftPlayer {
            whispering: true,
            ..p
        }),
        other => other,
    }
}

async fn choose_whisper(cm: &CacheManager, owner: &str, enabled: bool) {
    cm.preferences()
        .set(
            PreferenceKey::new(owner, WHISPER_CONTROL_TARGET),
            WhisperPreference::for_owner(owner, enabled),
        )
        .await;
}

async fn seed(cm: &CacheManager, speaker: PlayerEnum) {
    cm.players()
        .set(speaker.identity().to_string(), speaker)
        .await;
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

// Off by default: a player who never touched the setting must sound the same crouched as
// standing, whichever source reported the crouch.
#[tokio::test]
async fn a_whisper_without_a_preference_resolves_as_normal_voice() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(!speaker.is_whispering());
}

#[tokio::test]
async fn a_whisper_with_the_preference_on_stays_a_whisper() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;
    choose_whisper(&cm, "minecraft:Alice", true).await;

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(speaker.is_whispering());
}

#[tokio::test]
async fn a_whisper_with_the_preference_off_resolves_as_normal_voice() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;
    choose_whisper(&cm, "minecraft:Alice", false).await;

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(!speaker.is_whispering());
}

// The preference is keyed on its owner. Matching on the target alone would let one player's
// choice shrink everybody's range.
#[tokio::test]
async fn a_preference_for_another_player_does_not_opt_this_one_in() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;
    choose_whisper(&cm, "minecraft:Bob", true).await;

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(!speaker.is_whispering());
}

// No preference on this server is off, whoever the sender is. A peer-link speaker has no
// preference here, so a crouch that arrives with their record is heard as normal voice.
#[tokio::test]
async fn a_relayed_whisper_without_a_preference_here_is_normal_voice() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;

    let identity: common::PlayerIdentity = "minecraft:Alice".parse().expect("identity");
    let frame =
        RoutingFixture::audio_packet_from_sender(whisperer("Alice"), PacketSender::relayed(identity));
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(!speaker.is_whispering());
}

// End to end through routing: the resolved speaker is what `route_audio_frame` ranges on, so a
// crouching player who never opted in reaches a listener well past the whisper range.
#[tokio::test]
async fn crouching_without_opting_in_reaches_a_listener_at_normal_range() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;
    let bob = RoutingFixture::player("Bob", 30.0, false);
    let cache = RoutingFixture::player_cache(&[whisperer("Alice"), bob]).await;

    let reg = ConnectionRegistry::new();
    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), "fp-2".to_string(), bob_tx)
        .expect("admitted");

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await;
    reg.route_audio_frame(&frame, speaker.as_ref(), &cache, 50.0, 10.0).await;

    assert!(RoutingFixture::delivered_spatial(&mut bob_rx).await.is_some());
}
