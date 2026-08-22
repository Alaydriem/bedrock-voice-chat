use bvc_server_lib::services::metrics_service::interaction::InteractionRoute;
use bvc_server_lib::stream::quic::connection::{CapacityPolicy, ConnectionRegistry};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::harness::RoutingFixture;

/// A disconnect clears channel membership at once, not on the reaper's schedule.
///
/// The removal keys on `ConnectionEntry.identity`. Keyed on a bare gamertag against a map of
/// `game:gamertag`, it matched nothing and the entry survived until the grace sweep — which
/// reads exactly like the grace working as intended.
#[test]
fn a_disconnect_clears_channel_membership() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alaydriem".into(), format!("fp-{}", 1), tx).expect("admitted");
    reg.update_player_channel("minecraft:Alaydriem", "abc");
    assert_eq!(reg.channel_membership_count(), 1);

    reg.unregister(1);

    assert_eq!(reg.channel_membership_count(), 0);
}

/// A late close for a superseded connection must not evict the live one.
///
/// Reconnects arrive before the old connection's close on this branch's own recovery path. The
/// name index was already guarded against this; the channel membership was not, so the player
/// stayed in the channel by the server's own reckoning and heard proximity audio instead.
#[test]
fn a_stale_disconnect_leaves_a_reconnected_players_membership_alone() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alaydriem".into(), format!("fp-{}", 1), tx.clone()).expect("admitted");
    reg.try_register(2, "minecraft:Alaydriem".into(), format!("fp-{}", 2), tx).expect("admitted");
    reg.update_player_channel("minecraft:Alaydriem", "abc");

    reg.unregister(1);

    assert_eq!(reg.channel_membership_count(), 1);
}

#[test]
fn reaper_purges_absent_channel_membership_after_grace() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alice".into(), format!("fp-{}", 1), tx).expect("admitted");
    reg.update_player_channel("minecraft:Alice", "chan1");
    reg.update_player_channel("minecraft:Ghost", "chan2");
    assert_eq!(reg.channel_membership_count(), 2);

    // Ghost has no live connection; the grace is 2 sweeps, so one sweep keeps it.
    reg.reap_stale_channels();
    assert_eq!(reg.channel_membership_count(), 2);

    // Second sweep past grace purges Ghost; Alice (live) is never reaped.
    reg.reap_stale_channels();
    assert_eq!(reg.channel_membership_count(), 1);
}

#[test]
fn reaper_resets_grace_when_player_reconnects() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.update_player_channel("minecraft:Bob", "chan1");

    reg.reap_stale_channels(); // sweep 1 while Bob is absent
    reg.try_register(9, "minecraft:Bob".into(), format!("fp-{}", 9), tx).expect("admitted"); // Bob reconnects
    reg.reap_stale_channels(); // Bob live now -> grace reset, not purged

    assert_eq!(reg.channel_membership_count(), 1);
}

#[tokio::test]
async fn same_channel_recipient_receives_channel_variant_at_any_distance() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let bob = RoutingFixture::player("Bob", 100_000.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob.clone()]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");
    reg.update_player_channel("minecraft:Alice", "chan1");
    reg.update_player_channel("minecraft:Bob", "chan1");

    let speaker = Some(alice.clone());
    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;

    assert_eq!(
        RoutingFixture::delivered_spatial(&mut bob_rx).await,
        Some(false),
        "channel members bypass spatial attenuation"
    );
}

// Channel membership is an identity relationship, not a spatial one. A player who
// joined a channel but has never appeared in the position cache — in the group, not
// yet in the game — must still be heard by the other members. The sender's and
// recipient's game now comes from the authenticated certificate held on the
// connection, so neither side needs position data to resolve its channel key.
#[tokio::test]
async fn same_channel_members_hear_each_other_without_position_data() {
    let reg = ConnectionRegistry::new();
    // Nobody has sent a position packet, so the cache is empty.
    let cache = RoutingFixture::player_cache(&[]).await;

    let (alice_tx, _alice_rx) = mpsc::channel(16);
    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(1, "minecraft:Alice".into(), format!("fp-{}", 1), alice_tx).expect("admitted");
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");
    reg.update_player_channel("minecraft:Alice", "chan1");
    reg.update_player_channel("minecraft:Bob", "chan1");

    let speaker: Option<common::PlayerEnum> = None;
    let packet = RoutingFixture::audio_packet_without_position("minecraft:Alice");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;

    assert_eq!(
        RoutingFixture::delivered_spatial(&mut bob_rx).await,
        Some(false),
        "a channel member with no position data must still receive the channel variant"
    );
}

// The proximity path genuinely needs coordinates, so a sender with no position and
// no shared channel must not be broadcast to everyone as a side effect of the fix.
#[tokio::test]
async fn positionless_sender_outside_a_channel_is_not_routed() {
    let reg = ConnectionRegistry::new();
    let bob = RoutingFixture::player("Bob", 0.0, false);
    let cache = RoutingFixture::player_cache(&[bob]).await;

    let (alice_tx, _alice_rx) = mpsc::channel(16);
    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(1, "minecraft:Alice".into(), format!("fp-{}", 1), alice_tx).expect("admitted");
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");

    let speaker: Option<common::PlayerEnum> = None;
    let packet = RoutingFixture::audio_packet_without_position("minecraft:Alice");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;

    assert_eq!(
        RoutingFixture::delivered_spatial(&mut bob_rx).await,
        None,
        "no channel and no position means nothing to route on"
    );
}

#[tokio::test]
async fn in_range_recipient_receives_spatial_variant() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let bob = RoutingFixture::player("Bob", 10.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob.clone()]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");

    let speaker = Some(alice.clone());
    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;

    assert_eq!(
        RoutingFixture::delivered_spatial(&mut bob_rx).await,
        Some(true),
        "proximity recipients get the spatial variant"
    );
}

#[tokio::test]
async fn out_of_range_recipient_receives_nothing() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let bob = RoutingFixture::player("Bob", 1_000.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob.clone()]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");

    let speaker = Some(alice.clone());
    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;

    assert_eq!(RoutingFixture::delivered_spatial(&mut bob_rx).await, None);
}

#[tokio::test]
async fn sender_does_not_receive_own_frame() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone()]).await;

    let (alice_tx, mut alice_rx) = mpsc::channel(16);
    reg.try_register(1, "minecraft:Alice".into(), format!("fp-{}", 1), alice_tx).expect("admitted");

    let speaker = Some(alice.clone());
    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;

    assert_eq!(RoutingFixture::delivered_spatial(&mut alice_rx).await, None);
}

#[tokio::test]
async fn deafened_sender_is_limited_to_deafen_distance() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, true);
    // 30 blocks: inside 1.73*50 (normal range) but outside 1.73*10 (deafen range).
    let bob = RoutingFixture::player("Bob", 30.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob.clone()]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");

    let speaker = Some(alice.clone());
    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 10.0).await;

    assert_eq!(RoutingFixture::delivered_spatial(&mut bob_rx).await, None);
}

fn metrics_for(dir_name: &str) -> std::sync::Arc<bvc_server_lib::services::MetricsService> {
    let dir = std::env::temp_dir().join(dir_name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().to_string();
    bvc_server_lib::runtime::ca_cert::CaCertManager::new(&path)
        .ensure(&[String::from("localhost")])
        .unwrap();
    let (svc, _posthog) = bvc_server_lib::services::MetricsService::new_shared(
        false,
        &path,
        "/nonexistent-cert.pem",
        Vec::new(),
        false,
        true,
        None,
    );
    svc
}

// Jukebox and webhook playback emit real AudioFrame packets under a synthetic
// owner that has no live connection. Counting those would let a server whose
// players never speak to each other report healthy reach from background music
// alone — the exact blind spot this measurement exists to remove.
#[tokio::test]
async fn synthetic_sender_audio_is_not_counted_as_an_interaction() {
    let reg = ConnectionRegistry::new();
    let metrics = metrics_for("bvc-registry-synthetic-ca");
    reg.set_metrics(metrics.clone());

    let jukebox = RoutingFixture::player("Jukebox", 0.0, false);
    let bob = RoutingFixture::player("Bob", 1.0, false);
    let cache = RoutingFixture::player_cache(&[jukebox.clone(), bob.clone()]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");

    let speaker = Some(jukebox.clone());
    let packet = RoutingFixture::audio_packet(jukebox, "Jukebox");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 30.0, 0.0).await;

    // The frame still reaches Bob; only the measurement declines to count it.
    assert!(
        RoutingFixture::delivered_spatial(&mut bob_rx).await.is_some(),
        "jukebox audio must still be delivered"
    );
    assert_eq!(
        metrics
            .interactions()
            .counts(InteractionRoute::Any)
            .reached,
        0,
        "a sender with no live connection is not a player interaction"
    );
}

#[tokio::test]
async fn audio_between_two_connected_players_counts_both() {
    let reg = ConnectionRegistry::new();
    let metrics = metrics_for("bvc-registry-twoplayer-ca");
    reg.set_metrics(metrics.clone());

    let alice = RoutingFixture::player("Alice", 0.0, false);
    let bob = RoutingFixture::player("Bob", 1.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob.clone()]).await;

    let (alice_tx, _alice_rx) = mpsc::channel(16);
    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(1, "minecraft:Alice".into(), format!("fp-{}", 1), alice_tx).expect("admitted");
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");

    let speaker = Some(alice.clone());
    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 30.0, 0.0).await;

    assert!(RoutingFixture::delivered_spatial(&mut bob_rx).await.is_some());
    assert_eq!(
        metrics
            .interactions()
            .counts(InteractionRoute::Proximity)
            .reached,
        2,
        "both the speaker and the listener are participants"
    );
}

// Revocation addresses a live session by the credential it was opened with, so one identity
// holding two connections on two certificates loses only the revoked one.
#[tokio::test]
async fn a_connection_is_addressable_by_its_certificate_fingerprint() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(8);

    reg.try_register(7, "minecraft:Steve".into(), "ab".repeat(32), tx).expect("admitted");

    assert_eq!(reg.device_for_fingerprint(&"ab".repeat(32)), Some(7));
    assert_eq!(reg.device_for_fingerprint(&"cd".repeat(32)), None);
}

#[tokio::test]
async fn unregistering_clears_the_fingerprint_index() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    reg.try_register(7, "minecraft:Steve".into(), "ab".repeat(32), tx).expect("admitted");

    reg.unregister(7);

    assert_eq!(reg.device_for_fingerprint(&"ab".repeat(32)), None);
}

// Two sessions for one player, on different certificates. Revoking one must leave the other
// addressable, which is the whole reason the index is keyed on the credential.
#[tokio::test]
async fn two_connections_for_one_identity_are_addressed_separately() {
    let reg = ConnectionRegistry::new();
    let (tx_a, _rx_a) = tokio::sync::mpsc::channel(8);
    let (tx_b, _rx_b) = tokio::sync::mpsc::channel(8);

    reg.try_register(1, "minecraft:Steve".into(), "aa".repeat(32), tx_a).expect("admitted");
    reg.try_register(2, "minecraft:Steve".into(), "bb".repeat(32), tx_b).expect("admitted");

    assert_eq!(reg.device_for_fingerprint(&"aa".repeat(32)), Some(1));
    assert_eq!(reg.device_for_fingerprint(&"bb".repeat(32)), Some(2));
}

// The speaker's PlayerEnum rides a heartbeat, not every frame: the client rebuilds
// position from the last attached state, so re-sending it 50x/s was pure wire weight.
#[tokio::test]
async fn sender_attaches_on_heartbeat_not_every_frame() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let bob = RoutingFixture::player("Bob", 1.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx).expect("admitted");

    let speaker = Some(alice.clone());
    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");

    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;
    tokio::time::sleep(common::net::PositionCadence::INTERVAL + std::time::Duration::from_millis(25))
        .await;
    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 50.0, 5.0).await;

    let first = RoutingFixture::delivered_frame(&mut bob_rx).await.unwrap();
    let second = RoutingFixture::delivered_frame(&mut bob_rx).await.unwrap();
    let third = RoutingFixture::delivered_frame(&mut bob_rx).await.unwrap();
    assert!(first.speaker.is_some(), "first frame carries the speaker state");
    assert!(second.speaker.is_none(), "a frame inside the interval is thinned");
    assert!(third.speaker.is_some(), "the heartbeat re-attaches");
}

// A limit counts identities, not connections. Everything below turns on that: the registry
// keys connections on a per-connection id, so a player who reconnects holds two entries for
// as long as it takes the old one to close.

#[test]
fn admits_up_to_the_limit_and_refuses_the_next_new_identity() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(2, Duration::from_secs(60)));
    let (tx, _rx) = mpsc::channel(4);

    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx.clone())
        .expect("first is under the limit");
    reg.try_register(2, "minecraft:Bob".into(), "fp-2".to_string(), tx.clone())
        .expect("second fills the limit");

    let refusal = reg
        .try_register(3, "minecraft:Carol".into(), "fp-3".to_string(), tx)
        .expect_err("the third must be refused");
    assert_eq!(refusal.limit, 2, "the refusal names the limit it hit");
}

#[test]
fn zero_never_refuses() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(0, Duration::from_secs(60)));
    let (tx, _rx) = mpsc::channel(4);

    for device in 1..=50u64 {
        reg.try_register(
            device,
            format!("minecraft:Player{device}").into(),
            format!("fp-{device}"),
            tx.clone(),
        )
        .expect("zero is unlimited");
    }
}

#[test]
fn an_unset_policy_never_refuses() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);

    for device in 1..=50u64 {
        reg.try_register(
            device,
            format!("minecraft:Player{device}").into(),
            format!("fp-{device}"),
            tx.clone(),
        )
        .expect("a registry with no policy installed admits everyone");
    }
}

// The reconnect case, and the reason a slot is an identity. The player's old connection has
// not closed yet, so a limit that counted connections would refuse them their own slot.
#[test]
fn a_reconnect_is_admitted_at_the_limit() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(1, Duration::from_secs(60)));
    let (tx, _rx) = mpsc::channel(4);

    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx.clone())
        .expect("Alice fills the limit");

    reg.try_register(2, "minecraft:Alice".into(), "fp-2".to_string(), tx)
        .expect("Alice's second connection is her own slot, not a new one");
}

#[test]
fn a_clean_disconnect_leaves_a_reservation() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(2, Duration::from_secs(60)));
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx)
        .unwrap();

    reg.unregister(1);

    assert_eq!(reg.reservation_count(), 1);
}

// The case the reservation exists for: the client closed the socket cleanly — an app quit,
// or Android tearing it down on backgrounding — so the slot freed at once.
#[test]
fn a_reserved_slot_is_held_against_a_newcomer() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(1, Duration::from_secs(60)));
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx.clone())
        .unwrap();
    reg.unregister(1);

    reg.try_register(2, "minecraft:Alice".into(), "fp-2".to_string(), tx)
        .expect("Alice returns to the slot she reserved");
}

#[test]
fn a_reservation_stops_holding_the_slot_after_the_grace() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(1, Duration::from_millis(50)));
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx.clone())
        .unwrap();
    reg.unregister(1);

    std::thread::sleep(Duration::from_millis(80));

    reg.try_register(2, "minecraft:Bob".into(), "fp-2".to_string(), tx)
        .expect("past the grace, the slot belongs to whoever wants it");
}

// Without displacement a server that emptied cleanly would refuse every newcomer for the
// whole grace window, against a server carrying nobody at all.
#[test]
fn a_newcomer_displaces_the_oldest_reservation_when_nothing_is_free() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(2, Duration::from_secs(60)));
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx.clone())
        .unwrap();
    reg.try_register(2, "minecraft:Bob".into(), "fp-2".to_string(), tx.clone())
        .unwrap();

    reg.unregister(1);
    std::thread::sleep(Duration::from_millis(10));
    reg.unregister(2);

    reg.try_register(3, "minecraft:Carol".into(), "fp-3".to_string(), tx.clone())
        .expect("Carol takes the oldest reservation rather than being refused");

    // Alice left first, so hers was the reservation given up. Bob's still stands.
    reg.try_register(4, "minecraft:Bob".into(), "fp-4".to_string(), tx.clone())
        .expect("the newer reservation survived the displacement");

    let refusal = reg
        .try_register(5, "minecraft:Dave".into(), "fp-5".to_string(), tx)
        .expect_err("both slots are live now, so there is nothing left to displace");
    assert_eq!(refusal.limit, 2);
}

// A late close for a superseded connection must not reserve a slot the player is using: the
// name index and channel membership are already guarded this way, and a reservation written
// here would count a live player against the limit twice.
#[test]
fn a_stale_disconnect_does_not_reserve_a_live_players_slot() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(2, Duration::from_secs(60)));
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx.clone())
        .unwrap();
    reg.try_register(2, "minecraft:Alice".into(), "fp-2".to_string(), tx)
        .unwrap();

    reg.unregister(1);

    assert_eq!(
        reg.reservation_count(),
        0,
        "Alice is still connected on device 2; nothing about her departed"
    );
}

#[test]
fn an_unlimited_registry_never_accumulates_reservations() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx)
        .unwrap();

    reg.unregister(1);

    assert_eq!(
        reg.reservation_count(),
        0,
        "a server with no limit has no slots to hold, so the map must stay empty"
    );
}

#[test]
fn the_reaper_drops_expired_reservations() {
    let reg = ConnectionRegistry::new();
    reg.set_capacity(CapacityPolicy::new(4, Duration::from_millis(50)));
    let (tx, _rx) = mpsc::channel(4);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx)
        .unwrap();
    reg.unregister(1);
    assert_eq!(reg.reservation_count(), 1);

    std::thread::sleep(Duration::from_millis(80));
    reg.reap_stale_channels();

    assert_eq!(reg.reservation_count(), 0);
}

// The counter is the half an operator acts on: it distinguishes a quiet server from one
// that turned people away, which is the signal that an instance has outgrown its size.
#[tokio::test]
async fn a_refusal_is_counted() {
    let reg = ConnectionRegistry::new();
    let metrics = metrics_for("bvc-registry-capacity-ca");
    reg.set_metrics(metrics.clone());
    reg.set_capacity(CapacityPolicy::new(1, Duration::from_secs(60)));
    let (tx, _rx) = mpsc::channel(4);

    reg.try_register(1, "minecraft:Alice".into(), "fp-1".to_string(), tx.clone())
        .unwrap();
    assert_eq!(metrics.capacity_refusals(), 0, "nobody has been turned away");

    reg.try_register(2, "minecraft:Bob".into(), "fp-2".to_string(), tx.clone())
        .expect_err("Bob is over the limit");

    assert_eq!(metrics.capacity_refusals(), 1);

    reg.try_register(3, "minecraft:Alice".into(), "fp-3".to_string(), tx)
        .expect("Alice reconnecting is not a refusal");

    assert_eq!(
        metrics.capacity_refusals(),
        1,
        "an admitted reconnect must not be counted as a refusal"
    );
}

// The speaker's identity rides the same heartbeat as their position. Every frame in
// between carries the device id alone, which is the whole bandwidth saving. Asserted on
// the decoded envelope rather than on a byte count, so a later field addition cannot
// silently reintroduce the identity.
#[tokio::test]
async fn a_frame_between_heartbeats_carries_only_the_device() {
    let reg = ConnectionRegistry::new();

    let alice = RoutingFixture::player("Alice", 0.0, false);
    let bob = RoutingFixture::player("Bob", 1.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob.clone()]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx)
        .expect("admitted");

    let speaker = Some(alice.clone());
    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");

    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 30.0, 0.0).await;
    let first = RoutingFixture::delivered_envelope(&mut bob_rx)
        .await
        .expect("first frame delivered");
    assert!(
        first.sender_identity().is_some(),
        "the first frame from a speaker always attaches the identity"
    );

    reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 30.0, 0.0).await;
    let second = RoutingFixture::delivered_envelope(&mut bob_rx)
        .await
        .expect("second frame delivered");
    assert!(
        second.sender_identity().is_none(),
        "a frame inside the attach interval must not repeat the identity"
    );
    assert_eq!(
        second.sender_device(),
        Some(1),
        "the receiver still needs the device to resolve the speaker"
    );
}

// Injected audio has no device a recipient could resolve, so eliding its name would leave
// the frame unattributable. It keeps the full sender however often it is sent.
#[tokio::test]
async fn injected_audio_keeps_its_full_sender() {
    let reg = ConnectionRegistry::new();

    let jukebox = RoutingFixture::player("Jukebox", 0.0, false);
    let bob = RoutingFixture::player("Bob", 1.0, false);
    let cache = RoutingFixture::player_cache(&[jukebox.clone(), bob.clone()]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), format!("fp-{}", 2), bob_tx)
        .expect("admitted");

    let speaker = Some(jukebox.clone());
    let packet = RoutingFixture::audio_packet(jukebox, "Jukebox");

    for round in 0..2 {
        reg.route_audio_frame(&packet, speaker.as_ref(), &cache, 30.0, 0.0).await;
        let delivered = RoutingFixture::delivered_envelope(&mut bob_rx)
            .await
            .expect("injected audio delivered");
        assert_eq!(
            delivered.sender_service(),
            Some("Jukebox"),
            "round {round} must still name the service"
        );
    }
}

// The ingress gate and the egress attach read the same per-speaker interval. If the gate
// advanced it, the egress would find the interval already spent, never attach, and no
// listener would ever receive a position — audible voice that never pans, which reads as a
// spatial audio bug rather than as this.
#[test]
fn the_attach_query_does_not_consume_the_interval() {
    let reg = ConnectionRegistry::new();
    let now = std::time::Instant::now();

    assert!(reg.sender_attach_pending("minecraft:Alaydriem", now));
    assert!(
        reg.sender_attach_pending("minecraft:Alaydriem", now),
        "querying twice must not spend the interval"
    );
    assert!(
        reg.sender_attach_due("minecraft:Alaydriem", now),
        "the egress must still find the first attach due"
    );
    assert!(
        !reg.sender_attach_due("minecraft:Alaydriem", now),
        "the egress consumes it"
    );
    assert!(
        !reg.sender_attach_pending("minecraft:Alaydriem", now),
        "and the gate now agrees it is spent"
    );
}
