use std::net::SocketAddr;

use bvc_server_lib::relay::AdvertisedAddress;

// The observed port is the source port of the OUTBOUND probe, not the forwarded
// inbound one. A NAT with endpoint-independent mapping happens to preserve the pinned
// port; one that rewrites does not, and advertising what was observed would name a
// port nothing listens on. The configured port is the one the operator forwarded.
#[test]
fn the_advertised_port_is_the_configured_one_not_the_observed_one() {
    let observed: SocketAddr = "203.0.113.10:51823".parse().expect("addr");

    let advertised =
        AdvertisedAddress::from_observation(Some(observed), Some(28284)).expect("advertises");

    assert_eq!(advertised.ip(), observed.ip());
    assert_eq!(advertised.port(), 28284);
}

// Without a pinned port there is nothing forwarded, so cross-internet peering cannot
// work and advertising an address would promise reachability this server does not
// have.
#[test]
fn nothing_is_advertised_without_a_configured_port() {
    let observed: SocketAddr = "203.0.113.10:51823".parse().expect("addr");

    assert_eq!(
        AdvertisedAddress::from_observation(Some(observed), None),
        None
    );
}

// A registry that could not observe a direct address vouches for nothing, and an
// address this server cannot stand behind is worse than none at all.
#[test]
fn nothing_is_advertised_when_the_registry_observed_nothing() {
    assert_eq!(AdvertisedAddress::from_observation(None, Some(28284)), None);
}
