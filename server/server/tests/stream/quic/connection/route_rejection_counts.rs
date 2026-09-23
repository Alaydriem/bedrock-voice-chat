use bvc_server_lib::services::metrics_service::interaction::RouteRejection;
use bvc_server_lib::stream::quic::connection::ConnectionRegistry;
use tokio::sync::mpsc;

use crate::harness::RoutingFixture;

const RANGE: f32 = 16.0;
const DEAFEN: f32 = 4.0;

// A speaker whose position is not cached is skipped for every proximity listener and no group
// member. Before this counter the skip was a bare `continue`, indistinguishable from out-of-range.
#[tokio::test]
async fn an_unresolved_speaker_is_counted_once_per_proximity_recipient() {
    let reg = ConnectionRegistry::new();
    let (alice_tx, _alice_rx) = mpsc::channel(8);
    let (bob_tx, mut bob_rx) = mpsc::channel(8);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".into(), alice_tx).expect("admitted");
    reg.try_register(2, "minecraft:Bob".into(), "fp-2".into(), bob_tx).expect("admitted");
    let cache = RoutingFixture::player_cache(&[RoutingFixture::player("Bob", 0.0, false)]).await;
    let packet = RoutingFixture::audio_packet_without_position("minecraft:Alice");

    reg.route_audio_frame(&packet, None, &cache, RANGE, DEAFEN).await;

    assert_eq!(reg.route_rejections().get(RouteRejection::SpeakerUnresolved), 1);
    assert!(RoutingFixture::delivered_frame(&mut bob_rx).await.is_none());
}

#[tokio::test]
async fn out_of_range_is_its_own_reason() {
    let reg = ConnectionRegistry::new();
    let (alice_tx, _alice_rx) = mpsc::channel(8);
    let (bob_tx, mut bob_rx) = mpsc::channel(8);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".into(), alice_tx).expect("admitted");
    reg.try_register(2, "minecraft:Bob".into(), "fp-2".into(), bob_tx).expect("admitted");
    let alice = RoutingFixture::player("Alice", 0.0, false);
    let bob = RoutingFixture::player("Bob", 10_000.0, false);
    let cache = RoutingFixture::player_cache(&[alice.clone(), bob]).await;
    let packet = RoutingFixture::audio_packet(alice.clone(), "minecraft:Alice");

    reg.route_audio_frame(&packet, Some(&alice), &cache, RANGE, DEAFEN).await;

    assert_eq!(reg.route_rejections().get(RouteRejection::OutOfRange), 1);
    assert_eq!(reg.route_rejections().get(RouteRejection::SpeakerUnresolved), 0);
    assert!(RoutingFixture::delivered_frame(&mut bob_rx).await.is_none());
}

// The channel branch never reads a position, so a positionless speaker in a group is delivered
// and nothing is counted. This is the asymmetry the field report describes.
#[tokio::test]
async fn a_channel_delivery_counts_no_rejection() {
    let reg = ConnectionRegistry::new();
    let (alice_tx, _alice_rx) = mpsc::channel(8);
    let (bob_tx, mut bob_rx) = mpsc::channel(8);
    reg.try_register(1, "minecraft:Alice".into(), "fp-1".into(), alice_tx).expect("admitted");
    reg.try_register(2, "minecraft:Bob".into(), "fp-2".into(), bob_tx).expect("admitted");
    reg.update_player_channel("minecraft:Alice", "grp");
    reg.update_player_channel("minecraft:Bob", "grp");
    let cache = RoutingFixture::player_cache(&[]).await;
    let packet = RoutingFixture::audio_packet_without_position("minecraft:Alice");

    reg.route_audio_frame(&packet, None, &cache, RANGE, DEAFEN).await;

    assert!(RoutingFixture::delivered_frame(&mut bob_rx).await.is_some());
    for reason in RouteRejection::ALL {
        assert_eq!(reg.route_rejections().get(reason), 0, "{reason:?} must be zero");
    }
}

#[tokio::test]
async fn a_frame_with_no_routable_sender_is_counted_once() {
    let reg = ConnectionRegistry::new();
    let cache = RoutingFixture::player_cache(&[]).await;
    let mut packet = RoutingFixture::audio_packet_without_position("minecraft:Alice");
    packet.sender = None;

    reg.route_audio_frame(&packet, None, &cache, RANGE, DEAFEN).await;

    assert_eq!(reg.route_rejections().get(RouteRejection::SenderUnknown), 1);
}
