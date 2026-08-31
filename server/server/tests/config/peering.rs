use bvc_server_lib::config::{PeerConfig, Server};

// The socket has to bind before any code can be redeemed, and a server that has never
// paired has no `peers` blocks. Binding on declared peers alone would make the first
// pairing impossible.
#[test]
fn peering_is_enabled_by_the_flag_with_no_declared_peers() {
    let mut config = Server::default();
    config.peering = true;

    assert!(config.peering_enabled());
}

// Every deployment that predates the flag has peers and no flag. Those keep binding
// without an edit.
#[test]
fn peering_stays_enabled_by_declared_peers_with_no_flag() {
    let mut config = Server::default();
    config.peering = false;
    config.peers.insert(
        "other".to_string(),
        PeerConfig {
            peerlink: "bvcpeerAAAA".to_string(),
            worlds: vec![],
            capabilities: PeerConfig::default_capabilities(),
        },
    );

    assert!(config.peering_enabled());
}

#[test]
fn peering_is_enabled_by_default() {
    let config = Server::default();

    assert!(config.peering_enabled());
}

// The only arrangement that leaves the peer socket unbound: an operator who cleared the
// flag and declared nobody. Anything less than both keeps it bound.
#[test]
fn peering_is_off_only_when_the_flag_is_cleared_and_no_peer_is_declared() {
    let mut config = Server::default();
    config.peering = false;

    assert!(config.peers.is_empty());
    assert!(!config.peering_enabled());
}
