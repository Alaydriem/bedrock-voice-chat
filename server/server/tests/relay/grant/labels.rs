use std::collections::HashMap;

use bvc_relay::node::PeerTicket;
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::GrantTable;
use iroh::{EndpointAddr, SecretKey};

// A key is enough to mint a link, and a distinct key per block is what keeps
// `from_config` from rejecting the pair as duplicates.
fn config(worlds: &[&str]) -> PeerConfig {
    let key = SecretKey::generate().public();

    PeerConfig {
        peerlink: PeerTicket::mint(&EndpointAddr::new(key)).expect("mint"),
        worlds: worlds.iter().map(|w| w.to_string()).collect(),
        capabilities: vec!["carry_speakers".to_string()],
    }
}

// The runtime warning names the peer that asked for a world, so the block label
// has to survive parsing. A key would be accurate and unreadable.
#[test]
fn configured_worlds_are_reported_against_their_block_label() {
    let mut map = HashMap::new();
    map.insert("svc-bridge".to_string(), config(&["overworld", "nether"]));
    map.insert("pinned".to_string(), config(&["end"]));

    let table = GrantTable::from_config(&map).expect("valid config");

    assert_eq!(
        table.configured_worlds(),
        vec![
            ("pinned".to_string(), "end".to_string()),
            ("svc-bridge".to_string(), "nether".to_string()),
            ("svc-bridge".to_string(), "overworld".to_string()),
        ]
    );
}

// The ordinary block names only a peer link. It filters nothing, so it asks for
// no world and can never be warned about one.
#[test]
fn a_block_with_no_worlds_filter_contributes_nothing() {
    let mut map = HashMap::new();
    map.insert("svc-bridge".to_string(), config(&[]));

    let table = GrantTable::from_config(&map).expect("valid config");

    assert!(table.configured_worlds().is_empty());
}
