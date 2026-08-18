use std::sync::Arc;
use std::time::Duration;

use bvc_relay::peer::session::Inbox;
use common::game_data::Dimension;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};

fn frame(marker: u8) -> VoiceFrame {
    VoiceFrame {
        speaker: PlayerEnum::Minecraft(MinecraftPlayer {
            name: "Alice".to_string(),
            coordinates: Coordinate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: Some("W1".to_string()),
            bridged_voice: false,
        }),
        sample_rate: 48000,
        opus: vec![marker],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    }
}

#[tokio::test]
async fn delivers_in_order() {
    let inbox = Inbox::new(4);
    inbox.push(frame(1));
    inbox.push(frame(2));

    assert_eq!(inbox.next().await.expect("frame").opus, vec![1]);
    assert_eq!(inbox.next().await.expect("frame").opus, vec![2]);
}

// A full queue drops the oldest rather than the newest. Voice that is already
// late is worth less than voice that is not, so the frame to lose is the one
// furthest behind.
#[tokio::test]
async fn a_full_inbox_drops_the_oldest() {
    let inbox = Inbox::new(2);
    inbox.push(frame(1));
    inbox.push(frame(2));
    inbox.push(frame(3));

    assert_eq!(inbox.next().await.expect("frame").opus, vec![2]);
    assert_eq!(inbox.next().await.expect("frame").opus, vec![3]);
}

// The constraint the whole SDK surface is shaped around: uniffi cannot cancel a
// parked call, so close has to be what ends it. A consumer that cannot end a
// parked read cannot be shut down.
#[tokio::test]
async fn close_wakes_a_parked_reader() {
    let inbox = Arc::new(Inbox::new(4));

    let reader = tokio::spawn({
        let inbox = Arc::clone(&inbox);
        async move { inbox.next().await }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    inbox.close();

    let parked = tokio::time::timeout(Duration::from_secs(2), reader)
        .await
        .expect("close must wake a parked reader")
        .expect("join");

    assert!(parked.is_none());
}

// Frames already queued when close lands are still worth delivering: the caller
// asked to stop reading, not to discard what already arrived.
#[tokio::test]
async fn close_drains_before_it_ends() {
    let inbox = Inbox::new(4);
    inbox.push(frame(1));
    inbox.close();

    assert_eq!(inbox.next().await.expect("frame").opus, vec![1]);
    assert!(inbox.next().await.is_none());
}

// A closed inbox takes nothing further, so a receive pump that has not noticed
// the close yet cannot resurrect the queue.
#[tokio::test]
async fn a_closed_inbox_accepts_nothing() {
    let inbox = Inbox::new(4);
    inbox.close();
    inbox.push(frame(1));

    assert!(inbox.next().await.is_none());
}
