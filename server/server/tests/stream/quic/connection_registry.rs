use bvc_server_lib::stream::quic::connection_registry::ConnectionRegistry;
use tokio::sync::mpsc;

use crate::harness::RoutingFixture;

#[test]
fn reaper_purges_absent_channel_membership_after_grace() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.register(vec![1], "Alice".to_string(), tx.clone());
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
    reg.register(vec![9], "Bob".to_string(), tx.clone()); // Bob reconnects
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
    reg.register(vec![2], "Bob".to_string(), bob_tx);
    reg.update_player_channel("minecraft:Alice".to_string(), "chan1".to_string());
    reg.update_player_channel("minecraft:Bob".to_string(), "chan1".to_string());

    let packet = RoutingFixture::audio_packet(alice, "Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 5.0).await;

    assert_eq!(
        RoutingFixture::delivered_spatial(&mut bob_rx).await,
        Some(false),
        "channel members bypass spatial attenuation"
    );
}

#[tokio::test]
async fn in_range_recipient_receives_spatial_variant() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let bob = RoutingFixture::player("Bob", 10.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob.clone()]).await;

    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.register(vec![2], "Bob".to_string(), bob_tx);

    let packet = RoutingFixture::audio_packet(alice, "Alice");
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
    reg.register(vec![2], "Bob".to_string(), bob_tx);

    let packet = RoutingFixture::audio_packet(alice, "Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 5.0).await;

    assert_eq!(RoutingFixture::delivered_spatial(&mut bob_rx).await, None);
}

#[tokio::test]
async fn sender_does_not_receive_own_frame() {
    let reg = ConnectionRegistry::new();
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone()]).await;

    let (alice_tx, mut alice_rx) = mpsc::channel(16);
    reg.register(vec![1], "Alice".to_string(), alice_tx);

    let packet = RoutingFixture::audio_packet(alice, "Alice");
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
    reg.register(vec![2], "Bob".to_string(), bob_tx);

    let packet = RoutingFixture::audio_packet(alice, "Alice");
    reg.route_audio_frame(&packet, &cache, 50.0, 10.0).await;

    assert_eq!(RoutingFixture::delivered_spatial(&mut bob_rx).await, None);
}
