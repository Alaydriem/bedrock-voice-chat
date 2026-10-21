use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bvc_server_lib::services::group_key_relay_service::{CloseOutcome, RelayOutcome};
use bvc_server_lib::services::GroupKeyRelayService;
use bytes::Bytes;
use common::errors::ProtectedGroupError;
use common::structs::group_key::{GroupKeyFrame, GroupKeyRecipient, SocketId};
use tokio::sync::mpsc;

const GROUP: &str = "ABCD2345";

fn identity(canonical: &str) -> common::PlayerIdentity {
    canonical.parse().expect("canonical identity")
}

async fn open(
    relay: &GroupKeyRelayService,
    who: &str,
) -> (SocketId, mpsc::Receiver<GroupKeyFrame>) {
    let (id, mut rx) = relay.open(GROUP, identity(who)).await;
    assert_eq!(rx.recv().await, Some(GroupKeyFrame::Welcome { socket: id }));
    (id, rx)
}

fn commit(epoch: u64, tag: u8) -> GroupKeyFrame {
    GroupKeyFrame::Commit {
        epoch,
        body: Bytes::from(vec![tag]),
    }
}

fn direct(to: GroupKeyRecipient) -> GroupKeyFrame {
    GroupKeyFrame::Direct {
        to,
        body: Bytes::from_static(b"pake"),
    }
}

fn announce() -> GroupKeyFrame {
    GroupKeyFrame::Broadcast(Bytes::from_static(b"bind"))
}

// MLS needs every member to apply commits in one order; the relay is that order. Which commit
// wins an epoch is for the members to decide — only they can tell a member's commit from a
// stranger's — so the relay passes every commit on, in sequence, and judges none of them.
#[tokio::test]
async fn every_commit_is_broadcast_in_order_to_every_socket() {
    let relay = GroupKeyRelayService::new();
    let (a, mut a_rx) = open(&relay, "minecraft:Alice").await;
    let (b, mut b_rx) = open(&relay, "minecraft:Bob").await;
    let now = Instant::now();

    assert_eq!(relay.handle(GROUP, a, commit(0, 1), now).await, RelayOutcome::Routed);
    assert_eq!(relay.handle(GROUP, b, commit(0, 2), now).await, RelayOutcome::Routed);

    for rx in [&mut a_rx, &mut b_rx] {
        assert_eq!(rx.recv().await, Some(commit(0, 1)));
        assert_eq!(rx.recv().await, Some(commit(0, 2)));
    }
}

// A socket that never announced a binding is a joiner or a stranger: neither can admit anyone,
// and handing it a joiner's PAKE only costs the joiner a timeout.
#[tokio::test]
async fn direct_any_skips_sockets_that_never_announced() {
    let relay = GroupKeyRelayService::new();
    let (member, mut member_rx) = open(&relay, "minecraft:Alice").await;
    let (_stranger, mut stranger_rx) = open(&relay, "minecraft:Mallory").await;
    let (joiner, _joiner_rx) = open(&relay, "minecraft:Bob").await;
    let now = Instant::now();
    relay.handle(GROUP, member, announce(), now).await;

    for _ in 0..3 {
        assert_eq!(
            relay.handle(GROUP, joiner, direct(GroupKeyRecipient::Any), now).await,
            RelayOutcome::Routed
        );
    }

    let mut delivered_to_member = 0;
    while let Ok(frame) = member_rx.try_recv() {
        if matches!(frame, GroupKeyFrame::Delivered { from, .. } if from == joiner) {
            delivered_to_member += 1;
        }
    }
    assert_eq!(delivered_to_member, 3);
    while let Ok(frame) = stranger_rx.try_recv() {
        assert!(
            !matches!(frame, GroupKeyFrame::Delivered { from, .. } if from == joiner),
            "a joiner's PAKE went to a socket that never announced"
        );
    }
}

#[tokio::test]
async fn direct_any_never_routes_back_to_the_sender() {
    let relay = GroupKeyRelayService::new();
    let (a, mut a_rx) = open(&relay, "minecraft:Alice").await;
    let (b, mut b_rx) = open(&relay, "minecraft:Bob").await;
    let now = Instant::now();
    relay.handle(GROUP, a, announce(), now).await;
    relay.handle(GROUP, b, announce(), now).await;
    while a_rx.try_recv().is_ok() {}
    while b_rx.try_recv().is_ok() {}

    for _ in 0..4 {
        assert_eq!(
            relay.handle(GROUP, a, direct(GroupKeyRecipient::Any), now).await,
            RelayOutcome::Routed
        );
    }

    for _ in 0..4 {
        assert_eq!(
            b_rx.recv().await,
            Some(GroupKeyFrame::Delivered {
                from: a,
                body: Bytes::from_static(b"pake"),
            })
        );
    }
    assert!(a_rx.try_recv().is_err());
}

#[tokio::test]
async fn direct_any_with_no_other_socket_is_no_admitter() {
    let relay = GroupKeyRelayService::new();
    let (a, _a_rx) = open(&relay, "minecraft:Alice").await;

    let outcome = relay
        .handle(GROUP, a, direct(GroupKeyRecipient::Any), Instant::now())
        .await;

    assert_eq!(outcome, RelayOutcome::NoAdmitter);
}

// Two joiners and no member: neither can admit the other, so the answer is "nobody online", not
// a timeout spent waiting on each other.
#[tokio::test]
async fn direct_any_between_joiners_only_is_no_admitter() {
    let relay = GroupKeyRelayService::new();
    let (bob, _bob_rx) = open(&relay, "minecraft:Bob").await;
    let (_carol, _carol_rx) = open(&relay, "minecraft:Carol").await;

    let outcome = relay
        .handle(GROUP, bob, direct(GroupKeyRecipient::Any), Instant::now())
        .await;

    assert_eq!(outcome, RelayOutcome::NoAdmitter);
}

#[tokio::test]
async fn close_announces_gone_and_reports_group_empty_on_the_last_socket() {
    let relay = GroupKeyRelayService::new();
    let (a, _a_rx) = open(&relay, "minecraft:Alice").await;
    let (b, mut b_rx) = open(&relay, "minecraft:Bob").await;

    assert_eq!(relay.close(GROUP, a).await, CloseOutcome::MembersRemain);
    assert_eq!(b_rx.recv().await, Some(GroupKeyFrame::Gone { member: a }));
    assert!(relay.has_sockets(GROUP).await);

    assert_eq!(relay.close(GROUP, b).await, CloseOutcome::GroupEmpty);
    assert!(!relay.has_sockets(GROUP).await);
}

#[tokio::test]
async fn displaced_socket_queue_is_closed() {
    let relay = GroupKeyRelayService::new();
    let (first, mut first_rx) = open(&relay, "minecraft:Alice").await;
    let (_bob, mut bob_rx) = open(&relay, "minecraft:Bob").await;

    let (second, _second_rx) = open(&relay, "minecraft:Alice").await;

    assert_ne!(first, second);
    assert_eq!(first_rx.recv().await, None);
    assert_eq!(bob_rx.recv().await, Some(GroupKeyFrame::Gone { member: first }));
}

#[tokio::test]
async fn frame_over_256_kib_is_dropped() {
    let relay = GroupKeyRelayService::new();
    let (a, _a_rx) = open(&relay, "minecraft:Alice").await;
    let (_b, mut b_rx) = open(&relay, "minecraft:Bob").await;
    let oversized = GroupKeyFrame::Broadcast(Bytes::from(vec![0u8; 256 * 1024 + 1]));

    let outcome = relay.handle(GROUP, a, oversized, Instant::now()).await;

    assert_eq!(outcome, RelayOutcome::Dropped(ProtectedGroupError::FrameTooLarge));
    assert!(b_rx.try_recv().is_err());
}

#[tokio::test]
async fn socket_over_rate_for_30s_is_closed() {
    let relay = GroupKeyRelayService::new();
    let (a, mut a_rx) = open(&relay, "minecraft:Alice").await;
    let (_b, _b_rx) = open(&relay, "minecraft:Bob").await;
    let start = Instant::now();
    let frame = || GroupKeyFrame::Broadcast(Bytes::from_static(b"x"));

    // 21 frames inside one second is one over the limit.
    for _ in 0..21 {
        relay.handle(GROUP, a, frame(), start).await;
    }
    assert_eq!(
        relay.handle(GROUP, a, frame(), start).await,
        RelayOutcome::Dropped(ProtectedGroupError::RateLimited)
    );

    // Still over the limit every second for 30 seconds.
    for second in 1..=30u64 {
        let now = start + Duration::from_secs(second);
        for _ in 0..21 {
            relay.handle(GROUP, a, frame(), now).await;
        }
    }

    // Closed: the sender is dropped, so the queue ends after anything already buffered.
    let drained = tokio::time::timeout(Duration::from_secs(1), async {
        while a_rx.recv().await.is_some() {}
    })
    .await;
    assert!(drained.is_ok(), "the over-limit socket was never closed");
    assert_ne!(
        relay
            .handle(GROUP, a, frame(), start + Duration::from_secs(31))
            .await,
        RelayOutcome::Routed
    );
}

#[tokio::test]
async fn client_sent_server_frames_are_dropped() {
    let relay = GroupKeyRelayService::new();
    let (a, _a_rx) = open(&relay, "minecraft:Alice").await;
    let (b, mut b_rx) = open(&relay, "minecraft:Bob").await;

    for forged in [
        GroupKeyFrame::Stale { epoch: 9 },
        GroupKeyFrame::Gone { member: b },
        GroupKeyFrame::Welcome { socket: b },
        GroupKeyFrame::Delivered {
            from: b,
            body: Bytes::from_static(b"x"),
        },
    ] {
        assert!(matches!(
            relay.handle(GROUP, a, forged, Instant::now()).await,
            RelayOutcome::Dropped(_)
        ));
    }
    assert!(b_rx.try_recv().is_err());
}

#[tokio::test(start_paused = true)]
async fn creator_deadline_fires_only_without_sockets() {
    let relay = Arc::new(GroupKeyRelayService::new());

    let fired_empty = Arc::new(AtomicBool::new(false));
    let flag = fired_empty.clone();
    relay.arm_creator_deadline("EMPTY234".to_string(), move || {
        flag.store(true, Ordering::SeqCst);
    });

    let fired_open = Arc::new(AtomicBool::new(false));
    let flag = fired_open.clone();
    let (_a, _a_rx) = open(&relay, "minecraft:Alice").await;
    relay.arm_creator_deadline(GROUP.to_string(), move || {
        flag.store(true, Ordering::SeqCst);
    });

    tokio::time::sleep(Duration::from_secs(59)).await;
    assert!(!fired_empty.load(Ordering::SeqCst));

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(fired_empty.load(Ordering::SeqCst));
    assert!(!fired_open.load(Ordering::SeqCst));
}

fn members_frame() -> GroupKeyFrame {
    GroupKeyFrame::Direct {
        to: GroupKeyRecipient::Members,
        body: Bytes::from_static(b"request"),
    }
}

// A join request goes to every member at once, so whoever is at their screen can check the
// joiner. It never echoes to the sender.
#[tokio::test]
async fn members_recipient_reaches_every_other_socket() {
    let relay = GroupKeyRelayService::new();
    let (joiner, mut joiner_rx) = open(&relay, "minecraft:Steve").await;
    let (a, mut a_rx) = open(&relay, "minecraft:Alice").await;
    let (b, mut b_rx) = open(&relay, "minecraft:Bob").await;
    let now = Instant::now();
    relay.handle(GROUP, a, announce(), now).await;
    relay.handle(GROUP, b, announce(), now).await;
    while a_rx.try_recv().is_ok() {}
    while b_rx.try_recv().is_ok() {}
    while joiner_rx.try_recv().is_ok() {}

    let outcome = relay.handle(GROUP, joiner, members_frame(), now).await;

    assert_eq!(outcome, RelayOutcome::Routed);
    let expected = GroupKeyFrame::Asked {
        from: joiner,
        identity: "minecraft:Steve".to_string(),
        body: Bytes::from_static(b"request"),
    };
    assert_eq!(a_rx.try_recv().ok(), Some(expected.clone()));
    assert_eq!(b_rx.try_recv().ok(), Some(expected));
    assert!(joiner_rx.try_recv().is_err());
}

#[tokio::test]
async fn members_recipient_is_rate_limited() {
    let relay = GroupKeyRelayService::new();
    let (joiner, _joiner_rx) = open(&relay, "minecraft:Steve").await;
    let (a, mut a_rx) = open(&relay, "minecraft:Alice").await;
    let now = Instant::now();
    relay.handle(GROUP, a, announce(), now).await;
    while a_rx.try_recv().is_ok() {}

    assert_eq!(
        relay.handle(GROUP, joiner, members_frame(), now).await,
        RelayOutcome::Routed
    );
    assert!(matches!(
        relay
            .handle(GROUP, joiner, members_frame(), now + Duration::from_secs(9))
            .await,
        RelayOutcome::Dropped(ProtectedGroupError::RateLimited)
    ));
    assert_eq!(
        relay
            .handle(GROUP, joiner, members_frame(), now + Duration::from_secs(10))
            .await,
        RelayOutcome::Routed
    );

    let mut delivered = 0;
    while a_rx.try_recv().is_ok() {
        delivered += 1;
    }
    assert_eq!(delivered, 2);
}

#[tokio::test]
async fn members_recipient_with_no_member_online_is_no_admitter() {
    let relay = GroupKeyRelayService::new();
    let (joiner, _joiner_rx) = open(&relay, "minecraft:Steve").await;

    let outcome = relay
        .handle(GROUP, joiner, members_frame(), Instant::now())
        .await;

    assert_eq!(outcome, RelayOutcome::NoAdmitter);
}
