use bvc_client_lib::LoginClientConfig;
use common::net::NetTimeouts;
use common::structs::reachability::AddressFamilyPreference;
use std::net::{IpAddr, Ipv4Addr};

// The defect. The literal this replaced was 5 seconds, against a server that spends up to
// `IDENTITY` talking to five Microsoft hosts in series before it can answer at all.
#[test]
fn the_login_budget_outlasts_the_server_identity_exchange() {
    let config = LoginClientConfig::new(AddressFamilyPreference::PreferIpv4);

    assert_eq!(config.timeout(), NetTimeouts::LOGIN);
    assert!(config.timeout() > NetTimeouts::IDENTITY);
}

// Without this, the login budget — now sized for upstream work — is what a player waits out
// to learn a server is not there.
#[test]
fn a_connect_gives_up_long_before_the_request_budget() {
    let config = LoginClientConfig::new(AddressFamilyPreference::PreferIpv4);

    assert_eq!(config.connect_timeout(), NetTimeouts::CONNECT);
    assert!(config.connect_timeout() < config.timeout());
}

// The pin `api::Client` already applies, for the reason recorded there: some Windows
// machines advertise an IPv6 stack they cannot route, and a login is the first request a new
// player ever makes.
#[test]
fn preferring_ipv4_pins_the_local_socket_to_ipv4() {
    let config = LoginClientConfig::new(AddressFamilyPreference::PreferIpv4);

    assert_eq!(
        config.local_address(),
        Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
    );
}

// Lifted only for a host where a probe has already seen IPv6 answer, so the connector may
// choose for itself.
#[test]
fn preferring_ipv6_leaves_the_family_to_the_connector() {
    let config = LoginClientConfig::new(AddressFamilyPreference::PreferIpv6);

    assert_eq!(config.local_address(), None);
}
