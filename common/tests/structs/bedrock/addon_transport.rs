use common::structs::bedrock::AddonTransport;

#[test]
fn no_net_keeps_the_proxy_feeding_state_in_band() {
    let transport = AddonTransport::NoNet;
    assert!(!transport.suppresses_position_feed());
    assert!(!transport.suppresses_in_band_rides());
}

#[test]
fn net_suppresses_both_in_band_paths() {
    let transport = AddonTransport::Net;
    assert!(transport.suppresses_position_feed());
    assert!(transport.suppresses_in_band_rides());
}

// The default must never suppress. A world wrongly treated as net loses voice
// features silently; a world wrongly treated as no-net only produces redundant
// chatter. This guards the safe direction.
#[test]
fn default_suppresses_nothing() {
    let transport = AddonTransport::default();
    assert!(!transport.suppresses_position_feed());
    assert!(!transport.suppresses_in_band_rides());
}
