use bvc_server_lib::stream::quic::PacketIdentityStamp;
use common::structs::control::QueryState;
use common::structs::packet::{
    PacketOwner, PacketType, QueryStatePacket, QuicNetworkPacket, QuicNetworkPacketData,
};

fn query_state_packet(
    owner_name: &str,
    client_id: Vec<u8>,
    reported_id: &str,
) -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::QueryState,
        owner: Some(PacketOwner {
            name: owner_name.to_string(),
            client_id,
        }),
        data: QuicNetworkPacketData::QueryState(QueryStatePacket::new(QueryState {
            id: reported_id.to_string(),
            muted: false,
            deafened: false,
            recording: false,
            current_group: None,
        })),
    }
}

// A client claiming someone else's name must be rewritten to its authenticated
// identity, so `get_author()` can never return the spoofed value.
#[test]
fn spoofed_owner_name_is_replaced_with_the_authenticated_name() {
    let mut packet = query_state_packet("Victim", vec![1, 2, 3], "Victim");
    PacketIdentityStamp::apply(&mut packet, "Attacker");
    assert_eq!(packet.get_author(), "Attacker");
}

// `get_author()` falls back to base64(client_id) when the name is "api" or empty,
// which would let a client borrow the identity shape used by server-injected
// packets. Stamping must close that before the fallback is ever reached.
#[test]
fn api_owner_name_cannot_borrow_the_internal_author_shape() {
    let mut packet = query_state_packet("api", vec![9, 9, 9], "api");
    PacketIdentityStamp::apply(&mut packet, "Steve");
    assert_eq!(packet.get_author(), "Steve");
}

#[test]
fn empty_owner_name_is_replaced_with_the_authenticated_name() {
    let mut packet = query_state_packet("", vec![7], "");
    PacketIdentityStamp::apply(&mut packet, "Steve");
    assert_eq!(packet.get_author(), "Steve");
}

// client_id is the per-device routing key and must survive stamping untouched, so
// one player on two devices stays distinguishable.
#[test]
fn client_id_is_preserved_exactly() {
    let mut packet = query_state_packet("Steve", vec![4, 2, 4, 2], "Steve");
    PacketIdentityStamp::apply(&mut packet, "Steve");
    assert_eq!(packet.get_client_id(), vec![4, 2, 4, 2]);
}

// Two devices for the same authenticated player keep different client_ids, which is
// what lets the registry route to each independently.
#[test]
fn two_devices_share_a_name_but_keep_distinct_client_ids() {
    let mut first = query_state_packet("Steve", vec![1], "Steve");
    let mut second = query_state_packet("Steve", vec![2], "Steve");
    PacketIdentityStamp::apply(&mut first, "Steve");
    PacketIdentityStamp::apply(&mut second, "Steve");

    assert_eq!(first.get_author(), second.get_author());
    assert_ne!(first.get_client_id(), second.get_client_id());
}

// An owner-less packet is unattributable; stamping must not invent a client_id for
// it, and `get_author()` stays empty so no guard can match it.
#[test]
fn ownerless_packet_is_left_unattributed() {
    let mut packet = query_state_packet("Steve", vec![1], "Steve");
    packet.owner = None;
    PacketIdentityStamp::apply(&mut packet, "Steve");
    assert!(packet.owner.is_none());
    assert_eq!(packet.get_author(), "");
}
