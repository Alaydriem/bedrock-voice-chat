use bvc_server_lib::services::metrics_service::interaction::InteractionRoute;
use bvc_server_lib::stream::quic::connection::ConnectionRegistry;
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
    reg.register(1, "minecraft:Alaydriem".to_string(), format!("fp-{}", 1), tx);
    reg.update_player_channel("minecraft:Alaydriem".to_string(), "abc".to_string());
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
    reg.register(1, "minecraft:Alaydriem".to_string(), format!("fp-{}", 1), tx.clone());
    reg.register(2, "minecraft:Alaydriem".to_string(), format!("fp-{}", 2), tx);
    reg.update_player_channel("minecraft:Alaydriem".to_string(), "abc".to_string());

    reg.unregister(1);

    assert_eq!(reg.channel_membership_count(), 1);
}

#[test]
fn reaper_purges_absent_channel_membership_after_grace() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.register(1, "minecraft:Alice".to_string(), format!("fp-{}", 1), tx);
    reg.update_player_channel("minecraft:Alice".to_string(), "chan1".to_string());
    reg.update_player_channel("minecraft:Ghost".to_string(), "chan2".to_string());
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
    reg.update_player_channel("minecraft:Bob".to_string(), "chan1".to_string());

    reg.reap_stale_channels(); // sweep 1 while Bob is absent
    reg.register(9, "minecraft:Bob".to_string(), format!("fp-{}", 9), tx); // Bob reconnects
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
    reg.register(2, "minecraft:Bob".to_string(), format!("fp-{}", 2), bob_tx);
    reg.update_player_channel("minecraft:Alice".to_string(), "chan1".to_string());
    reg.update_player_channel("minecraft:Bob".to_string(), "chan1".to_string());

    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 5.0).await;

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
    reg.register(1, "minecraft:Alice".to_string(), format!("fp-{}", 1), alice_tx);
    reg.register(2, "minecraft:Bob".to_string(), format!("fp-{}", 2), bob_tx);
    reg.update_player_channel("minecraft:Alice".to_string(), "chan1".to_string());
    reg.update_player_channel("minecraft:Bob".to_string(), "chan1".to_string());

    let packet = RoutingFixture::audio_packet_without_position("minecraft:Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 5.0).await;

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
    reg.register(1, "minecraft:Alice".to_string(), format!("fp-{}", 1), alice_tx);
    reg.register(2, "minecraft:Bob".to_string(), format!("fp-{}", 2), bob_tx);

    let packet = RoutingFixture::audio_packet_without_position("minecraft:Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 5.0).await;

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
    reg.register(2, "minecraft:Bob".to_string(), format!("fp-{}", 2), bob_tx);

    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 5.0).await;

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
    reg.register(2, "minecraft:Bob".to_string(), format!("fp-{}", 2), bob_tx);

    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 5.0).await;

    assert_eq!(RoutingFixture::delivered_spatial(&mut bob_rx).await, None);
}

#[tokio::test]
async fn sender_does_not_receive_own_frame() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone()]).await;

    let (alice_tx, mut alice_rx) = mpsc::channel(16);
    reg.register(1, "minecraft:Alice".to_string(), format!("fp-{}", 1), alice_tx);

    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 5.0).await;

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
    reg.register(2, "minecraft:Bob".to_string(), format!("fp-{}", 2), bob_tx);

    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 10.0).await;

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
    reg.register(2, "minecraft:Bob".to_string(), format!("fp-{}", 2), bob_tx);

    let packet = RoutingFixture::audio_packet(jukebox, "Jukebox");
    reg.route_audio_frame(&packet, &cache, 30.0, 0.0).await;

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
    reg.register(1, "minecraft:Alice".to_string(), format!("fp-{}", 1), alice_tx);
    reg.register(2, "minecraft:Bob".to_string(), format!("fp-{}", 2), bob_tx);

    let packet = RoutingFixture::audio_packet(alice, "minecraft:Alice");
    reg.route_audio_frame(&packet, &cache, 30.0, 0.0).await;

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

    reg.register(7, "minecraft:Steve".to_string(), "ab".repeat(32), tx);

    assert_eq!(reg.device_for_fingerprint(&"ab".repeat(32)), Some(7));
    assert_eq!(reg.device_for_fingerprint(&"cd".repeat(32)), None);
}

#[tokio::test]
async fn unregistering_clears_the_fingerprint_index() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    reg.register(7, "minecraft:Steve".to_string(), "ab".repeat(32), tx);

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

    reg.register(1, "minecraft:Steve".to_string(), "aa".repeat(32), tx_a);
    reg.register(2, "minecraft:Steve".to_string(), "bb".repeat(32), tx_b);

    assert_eq!(reg.device_for_fingerprint(&"aa".repeat(32)), Some(1));
    assert_eq!(reg.device_for_fingerprint(&"bb".repeat(32)), Some(2));
}
