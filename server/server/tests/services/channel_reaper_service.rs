use std::sync::Arc;
use std::time::Instant;

use bvc_server_lib::services::ChannelReaperService;
use bvc_server_lib::stream::quic::connection::ConnectionRegistry;
use bvc_server_lib::stream::quic::{PacketOrigin, WebhookReceiver};
use common::structs::channel::{Channel, ChannelCollection, ChannelEvents};
use common::structs::packet::{PacketType, QuicNetworkPacket, QuicNetworkPacketData};
use common::{Game, PlayerIdentity};
use tokio::sync::mpsc;

fn identity(gamertag: &str) -> PlayerIdentity {
    Game::Minecraft.membership_key(gamertag)
}

/// A group holding `members`, owned by the first of them.
async fn group_with(channels: &ChannelCollection, members: &[PlayerIdentity]) -> String {
    let channel = Channel::new("group".to_string(), members[0].clone());
    let id = channel.id();
    channels.insert(channel).await;
    for member in members {
        channels.add_player_to_channel(member, &id).await;
    }
    id
}

fn connect(registry: &ConnectionRegistry, device: u64, identity: &PlayerIdentity) {
    let (tx, _rx) = mpsc::channel(4);
    registry
        .try_register(
            device,
            identity.to_string().into(),
            format!("fp-{device}"),
            tx,
        )
        .expect("admitted");
}

/// A member sitting in a group is enough to keep it, however long they sit there.
///
/// The `ChannelCollection` TTL resets on membership *writes*, so a stable group produces no
/// writes and ages out under its own members. The reaper asks whether anyone is connected
/// instead, which is the quantity that actually decides whether a group is in use.
#[tokio::test]
async fn a_group_with_a_connected_member_is_never_pruned() {
    let channels = Arc::new(ChannelCollection::new(16));
    let registry = Arc::new(ConnectionRegistry::new());
    let (packets, _rx) = mpsc::unbounded_channel();
    let reaper =
        ChannelReaperService::new(channels.clone(), registry.clone(), WebhookReceiver::new(packets));

    let alice = identity("Alice");
    connect(&registry, 1, &alice);
    let id = group_with(&channels, &[alice]).await;

    let start = Instant::now();
    reaper.sweep_at(start).await;
    reaper
        .sweep_at(start + ChannelReaperService::IDLE_TIMEOUT * 4)
        .await;

    assert!(
        channels.get(&id).await.is_some(),
        "a group with a connected member must survive any number of sweeps"
    );
}

/// The timeout is a grace, not a deadline on the first sweep that finds nobody home.
#[tokio::test]
async fn a_group_without_a_connected_member_survives_until_the_timeout() {
    let channels = Arc::new(ChannelCollection::new(16));
    let registry = Arc::new(ConnectionRegistry::new());
    let (packets, mut rx) = mpsc::unbounded_channel::<(QuicNetworkPacket, PacketOrigin)>();
    let reaper =
        ChannelReaperService::new(channels.clone(), registry.clone(), WebhookReceiver::new(packets));

    let id = group_with(&channels, &[identity("Ghost")]).await;

    let start = Instant::now();
    reaper.sweep_at(start).await;
    reaper
        .sweep_at(start + ChannelReaperService::IDLE_TIMEOUT / 2)
        .await;

    assert!(
        channels.get(&id).await.is_some(),
        "the group must survive inside the grace"
    );
    assert!(
        rx.try_recv().is_err(),
        "nothing is fanned while the group is still inside its grace"
    );
}

/// Past the grace the group is both removed and announced, so no client keeps showing a
/// group this server has forgotten.
#[tokio::test]
async fn a_group_past_the_timeout_is_removed_and_a_delete_is_fanned() {
    let channels = Arc::new(ChannelCollection::new(16));
    let registry = Arc::new(ConnectionRegistry::new());
    let (packets, mut rx) = mpsc::unbounded_channel::<(QuicNetworkPacket, PacketOrigin)>();
    let reaper =
        ChannelReaperService::new(channels.clone(), registry.clone(), WebhookReceiver::new(packets));

    let owner = identity("Ghost");
    let id = group_with(&channels, &[owner.clone()]).await;

    let start = Instant::now();
    reaper.sweep_at(start).await;
    reaper
        .sweep_at(start + ChannelReaperService::IDLE_TIMEOUT)
        .await;

    assert!(
        channels.get(&id).await.is_none(),
        "the group must be removed once the grace has run out"
    );

    let (packet, _origin) = rx.try_recv().expect("a Delete is fanned for the pruned group");
    assert_eq!(packet.packet_type, PacketType::ChannelEvent);
    let QuicNetworkPacketData::ChannelEvent(event) = packet.data else {
        panic!("the fanned packet must carry a ChannelEvent");
    };
    assert_eq!(event.event, ChannelEvents::Delete);
    assert_eq!(event.channel, id);
    assert_eq!(event.creator, Some(owner));
}

/// An empty group is one special case of the same rule, not a second rule.
///
/// It is also the common one: the desktop client leaves with `close_if_empty: false`, and a
/// disconnect fans a `Leave` without ever closing what it emptied.
#[tokio::test]
async fn an_empty_group_is_pruned_by_the_same_rule() {
    let channels = Arc::new(ChannelCollection::new(16));
    let registry = Arc::new(ConnectionRegistry::new());
    let (packets, _rx) = mpsc::unbounded_channel();
    let reaper =
        ChannelReaperService::new(channels.clone(), registry.clone(), WebhookReceiver::new(packets));

    let channel = Channel::new("abandoned".to_string(), identity("Owner"));
    let id = channel.id();
    channels.insert(channel).await;

    let start = Instant::now();
    reaper.sweep_at(start).await;
    reaper
        .sweep_at(start + ChannelReaperService::IDLE_TIMEOUT)
        .await;

    assert!(channels.get(&id).await.is_none());
}

/// A member who returns mid-grace restarts it. Without this the group dies at a fixed 15
/// minutes after the first absent sweep, however many times somebody came back in between.
#[tokio::test]
async fn a_member_returning_mid_grace_restarts_the_clock() {
    let channels = Arc::new(ChannelCollection::new(16));
    let registry = Arc::new(ConnectionRegistry::new());
    let (packets, _rx) = mpsc::unbounded_channel();
    let reaper =
        ChannelReaperService::new(channels.clone(), registry.clone(), WebhookReceiver::new(packets));

    let bob = identity("Bob");
    let id = group_with(&channels, &[bob.clone()]).await;

    let start = Instant::now();
    reaper.sweep_at(start).await;

    connect(&registry, 9, &bob);
    reaper
        .sweep_at(start + ChannelReaperService::IDLE_TIMEOUT / 2)
        .await;

    registry.unregister(9);
    reaper
        .sweep_at(start + ChannelReaperService::IDLE_TIMEOUT)
        .await;

    assert!(
        channels.get(&id).await.is_some(),
        "the grace restarts from Bob's return, so the original deadline must not prune"
    );

    reaper
        .sweep_at(start + ChannelReaperService::IDLE_TIMEOUT * 2)
        .await;

    assert!(
        channels.get(&id).await.is_none(),
        "the restarted grace still expires"
    );
}
