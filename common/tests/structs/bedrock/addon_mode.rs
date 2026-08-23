use common::structs::bedrock::AddonMode;

#[test]
fn net_relays_only() {
    assert!(AddonMode::Net.relays_only());
}

#[test]
fn no_net_processes_events() {
    assert!(!AddonMode::NoNet.relays_only());
}

// The default decides what an unmatched target does, so it is a behavioral
// contract rather than a derive. A typed-in address with no advertised entry and
// no override becomes a dumb relay.
#[test]
fn default_relays_only() {
    assert!(AddonMode::default().relays_only());
}
