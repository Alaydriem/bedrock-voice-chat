use bvc_relay::node::{PeerTicket, PeerTicketError};
use iroh::{EndpointAddr, SecretKey};

fn an_addr() -> EndpointAddr {
    // A fixed key, so the pinned encoding below is reproducible.
    let secret = SecretKey::from_bytes(&[7u8; 32]);
    EndpointAddr::new(secret.public())
}

// The ticket is a cross-version contract: an operator pastes it into a bridge
// built from a different checkout, and a change to the prefix, the alphabet or
// the payload layout silently turns every existing ticket into a parse error.
#[test]
fn a_ticket_encodes_to_its_pinned_string() {
    let ticket = PeerTicket::mint(&an_addr()).expect("mint");

    assert_eq!(
        ticket,
        "bvcpeer5jfgyy7ctrjavpxvkb5rglwf7gkuo5vox27hxescd3vgsfcg2iwaa"
    );
}

#[test]
fn a_minted_ticket_parses_back_to_the_same_endpoint() {
    let addr = an_addr();

    let parsed = PeerTicket::parse(&PeerTicket::mint(&addr).expect("mint")).expect("parse");

    assert_eq!(parsed.id, addr.id);
}

// The relay URL is the half that makes a ticket dialable off the local network.
#[test]
fn a_relay_url_survives_the_round_trip() {
    let addr = an_addr().with_relay_url("https://relay.example.".parse().expect("url"));

    let parsed = PeerTicket::parse(&PeerTicket::mint(&addr).expect("mint")).expect("parse");

    assert_eq!(
        parsed.relay_urls().next().map(|u| u.to_string()),
        Some("https://relay.example./".to_string())
    );
}

// An operator who pastes a node id where a ticket belongs must be told, not
// left with a peer that never connects.
#[test]
fn a_string_without_the_prefix_is_refused() {
    assert!(matches!(
        PeerTicket::parse("k51qzi5uqu5dabcdef"),
        Err(PeerTicketError::Prefix)
    ));
}

#[test]
fn a_ticket_with_a_character_outside_the_alphabet_is_refused() {
    assert!(matches!(
        PeerTicket::parse("bvcpeer!!!!"),
        Err(PeerTicketError::Alphabet)
    ));
}

// A ticket truncated in transit decodes to bytes that are not an endpoint.
#[test]
fn a_truncated_ticket_is_refused() {
    let ticket = PeerTicket::mint(&an_addr()).expect("mint");
    let truncated = &ticket[..ticket.len() - 8];

    assert!(matches!(
        PeerTicket::parse(truncated),
        Err(PeerTicketError::Payload) | Err(PeerTicketError::Alphabet)
    ));
}
