use bvc_server_lib::stream::quic::{CacheManager, CacheTrait};
use common::structs::channel::{Channel, ChannelEvents};
use common::structs::packet::{
    ChannelEventPacket, PacketSender, PacketType, PlayerDataPacket, QuicNetworkPacket,
    QuicNetworkPacketData,
};

use crate::harness::RoutingFixture;

const VICTIM: &str = "minecraft:Victim";
const ATTACKER: &str = "minecraft:Attacker";

// A packet the server injected: no connection minted it, so it carries no device.
fn from_server(packet_type: PacketType, data: QuicNetworkPacketData) -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type,
        data,
        sender: Some(PacketSender::for_service(PacketSender::SERVER_API)),
        ..Default::default()
    }
}

// A packet the ingress stamped from a player's certificate, which is the only way a
// device id is ever present.
fn from_player(
    packet_type: PacketType,
    data: QuicNetworkPacketData,
    identity: &str,
) -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type,
        data,
        sender: Some(PacketSender::player(
            identity.parse().expect("canonical identity"),
            1,
        )),
        ..Default::default()
    }
}

// `identity()` derives `game:name`, so the fixture takes the bare gamertag.
fn player_data(name: &str) -> QuicNetworkPacketData {
    QuicNetworkPacketData::PlayerData(PlayerDataPacket::new(vec![RoutingFixture::player(
        name, 5.0, false,
    )]))
}

fn identity(canonical: &str) -> common::PlayerIdentity {
    canonical.parse().expect("canonical identity")
}

fn channel_event(name: &str, channel_id: &str) -> QuicNetworkPacketData {
    QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new(
        ChannelEvents::Join,
        identity(name),
        channel_id.to_string(),
        None,
        Some(identity("minecraft:Owner")),
    ))
}

// `add_player_to_channel` is a no-op against a channel that does not exist, so a join has
// nothing to prove unless one is there to join.
async fn seeded_channel(cm: &CacheManager) -> String {
    let channel = Channel::new("Private".to_string(), identity("minecraft:Owner"));
    let id = channel.id();
    cm.get_channel_collection().insert(channel).await;
    id
}

// PlayerData keys the position cache from the names inside its body rather than from the
// sender, so a player connection able to send one writes any player's coordinates. Those
// coordinates are what proximity routing resolves, which makes this the difference between
// hearing your neighbours and hearing anyone you name.
#[tokio::test]
async fn player_data_from_a_player_connection_is_dropped() {
    let cm = CacheManager::new();

    cm.process_packet(from_player(
        PacketType::PlayerData,
        player_data("Victim"),
        ATTACKER,
    ))
    .await
    .expect("process");

    assert!(
        cm.players().get(&VICTIM.to_string()).await.is_none(),
        "a player connection must not write another player's coordinates"
    );
}

// The webhook path carries positions a mod reported, and those legitimately name many
// players at once. Rejecting by sender rather than by shape has to leave that intact.
#[tokio::test]
async fn player_data_from_the_server_is_applied() {
    let cm = CacheManager::new();

    cm.process_packet(from_server(PacketType::PlayerData, player_data("Victim")))
        .await
        .expect("process");

    assert!(
        cm.players().get(&VICTIM.to_string()).await.is_some(),
        "server-injected position data must still populate the cache"
    );
}

// Channel membership bypasses the proximity gate entirely, so a player connection able to
// send ChannelEvent can join itself to any conversation or evict anyone from theirs.
#[tokio::test]
async fn channel_event_from_a_player_connection_is_dropped() {
    let cm = CacheManager::new();
    let channel_id = seeded_channel(&cm).await;

    cm.process_packet(from_player(
        PacketType::ChannelEvent,
        channel_event(VICTIM, &channel_id),
        ATTACKER,
    ))
    .await
    .expect("process");

    assert!(
        cm.get_channel_collection()
            .get_player_channels(&identity(VICTIM))
            .is_empty(),
        "a player connection must not move another player between channels"
    );
}

// The channel API acts on a player's behalf, so the name in the body is somebody other than
// the sender by design. That path has no device and must keep working.
#[tokio::test]
async fn channel_event_from_the_server_is_applied() {
    let cm = CacheManager::new();
    let channel_id = seeded_channel(&cm).await;

    cm.process_packet(from_server(
        PacketType::ChannelEvent,
        channel_event(VICTIM, &channel_id),
    ))
    .await
    .expect("process");

    assert_eq!(
        cm.get_channel_collection().get_player_channels(&identity(VICTIM)),
        vec![channel_id],
        "the channel API must still be able to move a player it names"
    );
}
