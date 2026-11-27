use bvc_server_lib::services::{AudioPlaybackService, BedrockEventService};
use bvc_server_lib::stream::quic::WebhookReceiver;
use common::game_data::Dimension;
use common::structs::packet::{
    BedrockEvent, BedrockEventDirection, BedrockEventPacket, QuicNetworkPacket,
    QuicNetworkPacketData,
};
use common::{Coordinate, MinecraftPlayer, PlayerEnum};
use sea_orm::Database;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

async fn service_and_published() -> (
    BedrockEventService,
    mpsc::UnboundedReceiver<(QuicNetworkPacket, bvc_server_lib::stream::quic::PacketOrigin)>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let playback = Arc::new(AudioPlaybackService::new(
        WebhookReceiver::new(tx.clone()),
        ".".to_string(),
        CancellationToken::new(),
        1,
    ));
    let db = Arc::new(Database::connect("sqlite::memory:").await.unwrap());
    (
        BedrockEventService::new(playback, WebhookReceiver::new(tx), db, 30),
        rx,
    )
}

fn packet(event: BedrockEvent) -> BedrockEventPacket {
    BedrockEventPacket {
        event,
        world_uuid: "world-abc".to_string(),
        occurred_at_ms: 0,
        direction: BedrockEventDirection::ServerBound,
    }
}

async fn published_player(
    rx: &mut mpsc::UnboundedReceiver<(QuicNetworkPacket, bvc_server_lib::stream::quic::PacketOrigin)>,
) -> MinecraftPlayer {
    let (packet, _) = rx.recv().await.expect("a position packet");
    let QuicNetworkPacketData::PlayerData(data) = packet.data else {
        panic!("expected a player data packet");
    };
    match data.players.into_iter().next().expect("one player") {
        PlayerEnum::Minecraft(player) => player,
        _ => panic!("expected a minecraft player"),
    }
}

#[tokio::test]
async fn a_proxy_death_keeps_where_the_player_died() {
    let (svc, mut rx) = service_and_published().await;

    svc.handle_event(
        packet(BedrockEvent::PlayerDeath {
            player_xuid: "1".to_string(),
            dimension: Dimension::TheNether,
            last_pos: Coordinate {
                x: 10.0,
                y: 70.0,
                z: -4.0,
            },
        }),
        "Alice".to_string(),
    )
    .await
    .unwrap();

    let player = published_player(&mut rx).await;
    assert!(player.spectator);
    assert_eq!(player.dimension, Dimension::TheNether);
    assert_eq!(
        (player.coordinates.x, player.coordinates.y, player.coordinates.z),
        (10.0, 70.0, -4.0)
    );
}

#[tokio::test]
async fn a_proxy_leave_is_unrouted() {
    let (svc, mut rx) = service_and_published().await;

    svc.handle_event(
        packet(BedrockEvent::PlayerLeave {
            player_xuid: "1".to_string(),
        }),
        "Alice".to_string(),
    )
    .await
    .unwrap();

    let player = published_player(&mut rx).await;
    assert_eq!(player.dimension, Dimension::Unrouted);
}
