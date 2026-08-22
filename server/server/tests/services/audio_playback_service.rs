use bvc_server_lib::services::AudioPlaybackService;
use bvc_server_lib::stream::quic::WebhookReceiver;
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::{Coordinate, Orientation, PlayerEnum};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

fn service() -> AudioPlaybackService {
    let (tx, _rx) = mpsc::unbounded_channel();
    AudioPlaybackService::new(
        WebhookReceiver::new(tx),
        ".".to_string(),
        CancellationToken::new(),
        1,
    )
}

fn jukebox_player() -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: "jukebox-abcd1234".to_string(),
        coordinates: Coordinate {
            x: 10.0,
            y: 64.0,
            z: -5.0,
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

// The registry's expiry is derived from the track's own length, so a speaker registered for a
// long track is still resolvable at the end of it. A 15-second presence cache would not be, and
// no test on a short track can tell the two designs apart.
#[tokio::test]
async fn a_speaker_outlives_a_track_longer_than_the_presence_ttl() {
    let svc = service();
    svc.register_speaker(
        "jukebox-abcd1234".to_string(),
        jukebox_player(),
        Duration::from_secs(600),
    )
    .await;

    assert!(svc.speaker_for("jukebox-abcd1234").await.is_some());
}

// Cancelling a playback must stop its speaker resolving, or routing keeps placing audio at a
// block nothing is playing from.
#[tokio::test]
async fn cancelling_a_playback_stops_its_speaker_resolving() {
    let svc = service();
    svc.register_speaker(
        "jukebox-abcd1234".to_string(),
        jukebox_player(),
        Duration::from_secs(600),
    )
    .await;
    svc.forget_speaker("jukebox-abcd1234").await;

    assert!(svc.speaker_for("jukebox-abcd1234").await.is_none());
}

// A name nothing registered is not a speaker. Routing must treat that as "no position" rather
// than falling through to some default one.
#[tokio::test]
async fn an_unregistered_name_resolves_to_nothing() {
    let svc = service();
    assert!(svc.speaker_for("jukebox-never-played").await.is_none());
}
