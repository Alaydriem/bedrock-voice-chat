use std::sync::Arc;

use bvc_server_lib::services::{
    ChannelMembershipService, GroupLinkFeed, GroupLinkLeases, GroupLinkService,
};
use bvc_server_lib::stream::quic::{PacketOrigin, WebhookReceiver};
use common::structs::channel::ChannelCollection;
use common::structs::control::{GroupLinkAction, GroupLinkEventKind};
use common::structs::packet::QuicNetworkPacket;
use common::{Game, PlayerIdentity};
use tokio::sync::mpsc;

fn identity(gamertag: &str) -> PlayerIdentity {
    Game::Minecraft.membership_key(gamertag)
}

fn quiet_webhook() -> WebhookReceiver {
    let (packets, _rx) = mpsc::unbounded_channel::<(QuicNetworkPacket, PacketOrigin)>();
    WebhookReceiver::new(packets)
}

struct Fixture {
    channels: Arc<ChannelCollection>,
    webhook: WebhookReceiver,
    leases: Arc<GroupLinkLeases>,
    service: GroupLinkService,
}

fn fixture() -> Fixture {
    let channels = Arc::new(ChannelCollection::new(16));
    let webhook = quiet_webhook();
    let leases = GroupLinkLeases::new_shared();
    let service = GroupLinkService::new(
        channels.clone(),
        webhook.clone(),
        leases.clone(),
        GroupLinkFeed::new_shared(GroupLinkFeed::CAPACITY),
    );
    Fixture {
        channels,
        webhook,
        leases,
        service,
    }
}

fn publish_join(feed: &GroupLinkFeed, channel: &str, player: &str) {
    feed.publish(
        GroupLinkEventKind::Join,
        channel,
        Some("Raid".to_string()),
        Some(player.to_string()),
        false,
    );
}

#[test]
fn drain_returns_events_after_seq_in_order() {
    let feed = GroupLinkFeed::new(16);
    publish_join(&feed, "A", "Steve");
    publish_join(&feed, "A", "Alex");
    publish_join(&feed, "B", "Sam");

    let drain = feed.drain(1);

    assert!(!drain.reset);
    let seqs: Vec<u64> = drain.events.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, vec![2, 3]);
    assert_eq!(drain.events[0].player.as_deref(), Some("Alex"));
}

// A bridge that fell behind has missed events it can never read, so it must resync.
#[test]
fn drain_reports_reset_when_after_has_fallen_out() {
    let feed = GroupLinkFeed::new(2);
    for player in ["A", "B", "C", "D"] {
        publish_join(&feed, "grp", player);
    }

    let drain = feed.drain(0);

    assert!(drain.reset);
    let seqs: Vec<u64> = drain.events.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, vec![3, 4]);
}

// After a server restart the sequence starts over, so a bridge's last seen number is ahead of
// everything the feed holds.
#[test]
fn drain_reports_reset_when_after_is_ahead_of_the_feed() {
    let feed = GroupLinkFeed::new(16);
    publish_join(&feed, "grp", "Steve");

    assert!(feed.drain(50).reset);
}

#[test]
fn a_caught_up_drain_is_empty_and_not_reset() {
    let feed = GroupLinkFeed::new(16);
    publish_join(&feed, "grp", "Steve");

    let drain = feed.drain(1);

    assert!(!drain.reset);
    assert!(drain.events.is_empty());
}

#[test]
fn protected_channel_publishes_protected_then_nothing() {
    let feed = GroupLinkFeed::new(16);
    publish_join(&feed, "grp", "Steve");
    feed.publish(GroupLinkEventKind::Protected, "grp", None, None, false);
    publish_join(&feed, "grp", "Alex");
    feed.publish(GroupLinkEventKind::Delete, "grp", None, None, false);

    let kinds: Vec<GroupLinkEventKind> = feed.drain(0).events.iter().map(|e| e.kind).collect();

    assert_eq!(
        kinds,
        vec![GroupLinkEventKind::Join, GroupLinkEventKind::Protected]
    );
}

// A channel created protected was never linked; nothing about it may reach the bridge.
#[test]
fn a_silenced_channel_publishes_nothing() {
    let feed = GroupLinkFeed::new(16);
    feed.silence("sec");
    publish_join(&feed, "sec", "Steve");
    feed.publish(GroupLinkEventKind::Delete, "sec", None, None, false);

    assert!(feed.drain(0).events.is_empty());
}

#[tokio::test]
async fn create_linked_returns_the_new_channel_id() {
    let fx = fixture();

    let id = fx
        .service
        .apply(GroupLinkAction::CreateLinked {
            creator: "Steve".to_string(),
            name: "Raid".to_string(),
            members_only: true,
        })
        .await
        .expect("applied")
        .expect("an id");

    let channel = fx.channels.get(&id).await.expect("created");
    assert!(channel.is_members_only());
    assert_eq!(channel.creator, identity("Steve"));
}

#[tokio::test]
async fn close_refuses_protected_channel() {
    let fx = fixture();
    let id = ChannelMembershipService::create_protected(
        &fx.channels,
        &fx.webhook,
        "Secret".to_string(),
        identity("Steve"),
    )
    .await;

    let result = fx
        .service
        .apply(GroupLinkAction::Close {
            channel: id.clone(),
        })
        .await;

    assert!(result.is_err());
    assert!(fx.channels.get(&id).await.is_some());
}

#[tokio::test]
async fn close_removes_a_linked_channel() {
    let fx = fixture();
    let id = ChannelMembershipService::create_linked(
        &fx.channels,
        &fx.webhook,
        "Raid".to_string(),
        identity("Steve"),
        false,
    )
    .await;

    fx.service
        .apply(GroupLinkAction::Close {
            channel: id.clone(),
        })
        .await
        .expect("applied");

    assert!(fx.channels.get(&id).await.is_none());
}

#[tokio::test]
async fn channels_lists_non_protected_only() {
    let fx = fixture();
    let open = ChannelMembershipService::create(
        &fx.channels,
        &fx.webhook,
        "Open".to_string(),
        identity("Steve"),
    )
    .await;
    fx.channels.add_player_to_channel(&identity("Steve"), &open).await;
    ChannelMembershipService::create_protected(
        &fx.channels,
        &fx.webhook,
        "Secret".to_string(),
        identity("Alex"),
    )
    .await;

    let views = fx.service.channels();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].id, open);
    assert_eq!(views[0].players, vec!["Steve".to_string()]);
}

#[tokio::test]
async fn lease_action_renews_only_non_protected() {
    let fx = fixture();
    let open = ChannelMembershipService::create(
        &fx.channels,
        &fx.webhook,
        "Open".to_string(),
        identity("Steve"),
    )
    .await;
    let secret = ChannelMembershipService::create_protected(
        &fx.channels,
        &fx.webhook,
        "Secret".to_string(),
        identity("Alex"),
    )
    .await;

    fx.service
        .apply(GroupLinkAction::Lease {
            channels: vec![open.clone(), secret.clone(), "gone".to_string()],
        })
        .await
        .expect("applied");

    assert!(fx.leases.is_leased(&open));
    assert!(!fx.leases.is_leased(&secret));
    assert!(!fx.leases.is_leased("gone"));
}

// The bridge tells a restarted server from a quiet one by the epoch, so it must hold for the
// feed's whole life and fit the bridge's signed 64-bit and JSON number range.
#[test]
fn a_feed_reports_one_epoch_for_its_life() {
    let feed = GroupLinkFeed::new(16);
    let first = feed.drain(0).epoch;
    publish_join(&feed, "grp", "Steve");

    assert_eq!(feed.drain(0).epoch, first);
    assert!(first > 0);
    assert!(first < (1u64 << 53));
}

#[test]
fn a_new_feed_has_a_new_epoch() {
    assert_ne!(
        GroupLinkFeed::new(16).drain(0).epoch,
        GroupLinkFeed::new(16).drain(0).epoch
    );
}
