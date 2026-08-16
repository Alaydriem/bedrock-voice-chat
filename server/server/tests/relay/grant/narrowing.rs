use std::collections::HashMap;

use bvc_relay::node::PeerTicket;
use bvc_relay::peer::PeerAuthority;
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::GrantTable;
use iroh::{EndpointAddr, SecretKey};

fn a_ticket() -> (iroh::PublicKey, String) {
    let key = SecretKey::generate().public();
    (key, PeerTicket::mint(&EndpointAddr::new(key)).expect("mint"))
}

fn table_of(peerlink: String, worlds: &[&str]) -> GrantTable {
    let mut map = HashMap::new();
    map.insert(
        "bridge".to_string(),
        PeerConfig {
            peerlink,
            worlds: worlds.iter().map(|w| w.to_string()).collect(),
            capabilities: vec!["carry_speakers".to_string()],
        },
    );
    GrantTable::from_config(&map).expect("valid config")
}

// A block that names only a peer link trusts what the peer declares, which is
// the whole point: the operator never sees a relay world id.
#[test]
fn a_block_without_worlds_accepts_the_whole_declaration() {
    let (node, peerlink) = a_ticket();
    let table = table_of(peerlink, &[]);

    let scope = table
        .authorize(&node, &["alpha".to_string(), "beta".to_string()])
        .expect("authorized");

    assert_eq!(scope.worlds, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn a_block_with_worlds_narrows_the_declaration() {
    let (node, peerlink) = a_ticket();
    let table = table_of(peerlink, &["alpha"]);

    let scope = table
        .authorize(&node, &["alpha".to_string(), "beta".to_string()])
        .expect("authorized");

    assert_eq!(scope.worlds, vec!["alpha".to_string()]);
}

// The filter never adds. A world in config that the peer does not host is the
// operator's mistake, and inventing it would recreate the silent-drop failure.
#[test]
fn a_filter_never_grants_a_world_the_peer_did_not_declare() {
    let (node, peerlink) = a_ticket();
    let table = table_of(peerlink, &["alpha", "gamma"]);

    let scope = table
        .authorize(&node, &["alpha".to_string()])
        .expect("authorized");

    assert_eq!(scope.worlds, vec!["alpha".to_string()]);
}

#[test]
fn an_undeclared_node_is_refused_whatever_it_declares() {
    let (_, peerlink) = a_ticket();
    let table = table_of(peerlink, &[]);
    let stranger = SecretKey::generate().public();

    assert!(table.authorize(&stranger, &["alpha".to_string()]).is_none());
}

// An omitted capability list is the common case, and it must produce a working
// voice link rather than one that carries nothing.
#[test]
fn an_omitted_capability_list_grants_carry_speakers() {
    let (node, peerlink) = a_ticket();
    let mut map = HashMap::new();
    map.insert(
        "bridge".to_string(),
        PeerConfig {
            peerlink,
            worlds: Vec::new(),
            capabilities: PeerConfig::default_capabilities(),
        },
    );
    let table = GrantTable::from_config(&map).expect("valid config");

    assert!(table.may_carry(&node, "alpha"));
}

// An empty list is a deliberate statement, not an omission, so it is left empty
// — and a peer holding no capability carries nothing.
#[test]
fn an_explicitly_empty_capability_list_grants_nothing() {
    let (node, peerlink) = a_ticket();
    let mut map = HashMap::new();
    map.insert(
        "bridge".to_string(),
        PeerConfig {
            peerlink,
            worlds: Vec::new(),
            capabilities: Vec::new(),
        },
    );
    let table = GrantTable::from_config(&map).expect("valid config");

    assert!(!table.may_carry(&node, "alpha"));
}
