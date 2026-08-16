use std::collections::HashMap;

use bvc_relay::node::PeerTicket;
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::GrantTable;
use common::structs::relay::Capability;
use iroh::SecretKey;

// Returns the key and the peer link that carries it, because a block names the
// link while every assertion here is about the key inside it.
fn a_peer() -> (iroh::PublicKey, String) {
    let key = SecretKey::generate().public();
    (
        key,
        PeerTicket::mint(&iroh::EndpointAddr::new(key)).expect("mint"),
    )
}

fn block(peerlink: &str, worlds: &[&str], capabilities: &[&str]) -> PeerConfig {
    PeerConfig {
        peerlink: peerlink.to_string(),
        worlds: worlds.iter().map(|w| w.to_string()).collect(),
        capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
    }
}

fn table_of(entries: Vec<(&str, PeerConfig)>) -> GrantTable {
    let map: HashMap<String, PeerConfig> = entries
        .into_iter()
        .map(|(label, cfg)| (label.to_string(), cfg))
        .collect();
    GrantTable::from_config(&map).expect("config is valid")
}

#[test]
fn a_declared_peer_may_carry_speakers_for_a_granted_world() {
    let (node, id) = a_peer();
    let table = table_of(vec![("bridge", block(&id, &["W1"], &["carry_speakers"]))]);
    
    assert!(table.may_carry(&node, "W1"));
}

// The regression guard for the defect the old store had: `ActiveIdentity` held a
// single world and every write overwrote it, so a peer sharing two worlds
// silently lost one.
#[test]
fn a_grant_covering_two_worlds_covers_both() {
    let (node, id) = a_peer();
    let table = table_of(vec![(
        "bridge",
        block(&id, &["W1", "W2"], &["carry_speakers"]),
    )]);
    
    assert!(table.may_carry(&node, "W1"), "first world must be covered");
    assert!(table.may_carry(&node, "W2"), "second world must be covered");
}

#[test]
fn a_world_outside_the_grant_is_refused() {
    let (node, id) = a_peer();
    let table = table_of(vec![("bridge", block(&id, &["W1"], &["carry_speakers"]))]);
    
    assert!(!table.may_carry(&node, "W2"));
}

#[test]
fn an_undeclared_node_is_refused_entirely() {
    let (_declared_key, declared) = a_peer();
    let table = table_of(vec![(
        "bridge",
        block(&declared, &["W1"], &["carry_speakers"]),
    )]);
    let stranger = SecretKey::generate().public();

    assert!(table.grant_for(&stranger).is_none());
    assert!(!table.may_carry(&stranger, "W1"));
}

#[test]
fn a_capability_the_block_did_not_declare_is_refused() {
    let (node, id) = a_peer();
    let table = table_of(vec![("bridge", block(&id, &["W1"], &["carry_speakers"]))]);
    
    let grant = table.grant_for(&node).expect("declared");
    assert!(grant.allows(Capability::CarrySpeakers));
    assert!(
        !grant.allows(Capability::QueryAudio),
        "a capability must be declared to be held"
    );
}

// A peer declared without `carry_speakers` must not carry speakers, even for a
// world its block names.
#[test]
fn a_world_grant_without_carry_speakers_does_not_carry() {
    let (node, id) = a_peer();
    let table = table_of(vec![("bridge", block(&id, &["W1"], &["query_audio"]))]);
    
    assert!(!table.may_carry(&node, "W1"));
}

#[test]
fn an_unreadable_peerlink_fails_the_whole_config() {
    let mut map = HashMap::new();
    map.insert(
        "bridge".to_string(),
        block("not-a-peerlink", &["W1"], &["carry_speakers"]),
    );

    assert!(
        GrantTable::from_config(&map).is_err(),
        "a mangled peer link must stop startup, not silently grant nothing"
    );
}

#[test]
fn an_unrecognized_capability_fails_the_whole_config() {
    let (_node, id) = a_peer();
    let mut map = HashMap::new();
    map.insert("bridge".to_string(), block(&id, &["W1"], &["carry_speaker"]));

    assert!(
        GrantTable::from_config(&map).is_err(),
        "a misspelled capability must stop startup rather than be ignored"
    );
}

// Two labels naming one key is ambiguous: whichever loses would be silently
// unauthorized, and a HashMap gives no say in which that is.
#[test]
fn two_blocks_naming_the_same_node_fail_the_config() {
    let (_node, id) = a_peer();
    let mut map = HashMap::new();
    map.insert(
        "first".to_string(),
        block(&id, &["W1"], &["carry_speakers"]),
    );
    map.insert(
        "second".to_string(),
        block(&id, &["W2"], &["carry_speakers"]),
    );

    assert!(GrantTable::from_config(&map).is_err());
}

#[test]
fn an_empty_config_yields_an_empty_table() {
    let table = GrantTable::from_config(&HashMap::new()).expect("empty config is valid");

    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
}
