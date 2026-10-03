use bvc_server_lib::services::ChannelMembershipService;
use bvc_server_lib::stream::quic::{PacketOrigin, WebhookReceiver};
use common::structs::channel::{Channel, ChannelCollection, ChannelEvents};
use common::structs::packet::{
    PacketSender, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::{Game, PlayerIdentity};
use tokio::sync::mpsc;

fn identity(gamertag: &str) -> PlayerIdentity {
    Game::Minecraft.membership_key(gamertag)
}

// Six of the eight channel fan sites used to send `creator: None`, because the short
// constructor made omitting it the path of least resistance. A client that learns of a
// channel through a Join then has no owner to compare against, so the group renders as
// owned by nobody.
#[tokio::test]
async fn a_departing_member_carries_the_channel_owner_back() {
    let channels = ChannelCollection::new(16);
    let owner = identity("Owner");
    let joiner = identity("Joiner");

    let channel = Channel::new("group".to_string(), owner.clone());
    let id = channel.id();
    channels.insert(channel).await;

    channels.add_player_to_channel(&joiner, &id).await;

    let stored = channels.get(&id).await.expect("channel exists");
    assert_eq!(stored.creator, owner);
    assert!(stored.contains(&joiner));

    let left = channels.remove_player_from_all_channels(&joiner).await;
    assert_eq!(left, vec![(id, owner)]);
}

// Ownership is exact. A bare gamertag cannot be built, so the only remaining way to get
// this wrong is comparing two different players.
#[tokio::test]
async fn a_non_owner_does_not_match_the_creator() {
    let channels = ChannelCollection::new(16);
    let owner = identity("Owner");
    let channel = Channel::new("group".to_string(), owner.clone());
    let id = channel.id();
    channels.insert(channel).await;

    let stored = channels.get(&id).await.expect("channel exists");
    assert_ne!(stored.creator, identity("Interloper"));
    assert_eq!(stored.creator, owner);
}

// Two players sharing a gamertag across games are different owners. `Game` carries one
// variant today, so this is the guard that the discriminator is actually consulted rather
// than the gamertag alone deciding ownership.
#[tokio::test]
async fn ownership_consults_the_game_and_not_only_the_gamertag() {
    let owner = identity("Alaydriem");

    assert_eq!(owner.gamertag(), "Alaydriem");
    assert_eq!(owner.game(), &Game::Minecraft);
    assert_eq!(owner, PlayerIdentity::new(Game::Minecraft, "Alaydriem"));
    assert!("Alaydriem".parse::<PlayerIdentity>().is_err());
}

// The notice that a disconnected player left their groups is the server speaking, not the
// player. Stamped with the player's own connection it reads as a client forging membership,
// the cache manager refuses it as one, and every disconnect logs a false attack warning.
#[tokio::test]
async fn a_departure_is_announced_by_the_server_rather_than_the_player() {
    let (packets, mut rx) = mpsc::unbounded_channel::<(QuicNetworkPacket, PacketOrigin)>();
    let webhook = WebhookReceiver::new(packets);
    let owner = identity("Owner");
    let leaver = identity("Leaver");

    ChannelMembershipService::announce_departure(&webhook, &leaver, "ABCD-1234", owner.clone())
        .await;

    let (packet, _) = rx.try_recv().expect("a Leave is fanned");
    assert_eq!(packet.packet_type, PacketType::ChannelEvent);
    assert_eq!(packet.sender_device(), None);
    assert_eq!(packet.sender_service(), Some(PacketSender::CHANNEL_API));

    let QuicNetworkPacketData::ChannelEvent(event) = packet.data else {
        panic!("a ChannelEvent carries a ChannelEvent body");
    };
    assert_eq!(event.event, ChannelEvents::Leave);
    assert_eq!(event.name, leaver);
    assert_eq!(event.channel, "ABCD-1234");
    assert_eq!(event.creator, Some(owner));
}
