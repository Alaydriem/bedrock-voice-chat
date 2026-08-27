use bvc_relay_service::validation::RoutableAddress;

// A public address is probed, because an operator who declares one they do not
// control would otherwise have the relay fronting a third party's host from its own
// zone.
#[test]
fn a_routable_v4_address_is_public() {
    assert!(RoutableAddress::is_public("8.8.8.8"));
}

// The documentation ranges are reserved and route nowhere, so a server declaring one
// is misconfigured rather than reachable. Worth pinning because these are exactly the
// addresses that end up in a config file copied from an example.
#[test]
fn documentation_ranges_are_not_public() {
    for address in ["192.0.2.1", "198.51.100.1", "203.0.113.10"] {
        assert!(
            !RoutableAddress::is_public(address),
            "{address} must not be probed"
        );
    }
}

// Every range an operator behind NAT, CGNAT or on a LAN would declare. Published so
// their own network resolves it, never probed: unreachable from the relay by
// construction, and unable to front anyone because it resolves nowhere else.
#[test]
fn private_and_carrier_ranges_are_not_public() {
    for address in [
        "10.0.0.5",
        "172.16.4.1",
        "192.168.1.10",
        "127.0.0.1",
        "169.254.1.1",
        "100.64.0.1",
        "100.127.255.254",
        "0.0.0.0",
    ] {
        assert!(
            !RoutableAddress::is_public(address),
            "{address} must not be probed"
        );
    }
}

// The CGNAT block ends at 100.127.255.255. An address just past it is ordinary
// public space and must not be swept up by an over-wide mask.
#[test]
fn an_address_just_outside_the_carrier_range_is_public() {
    assert!(RoutableAddress::is_public("100.128.0.1"));
}

#[test]
fn v6_loopback_unique_local_and_link_local_are_not_public() {
    for address in ["::1", "fc00::1", "fd12:3456::1", "fe80::1", "::"] {
        assert!(
            !RoutableAddress::is_public(address),
            "{address} must not be probed"
        );
    }
}

#[test]
fn a_global_v6_address_is_public() {
    assert!(RoutableAddress::is_public("2606:4700:4700::1111"));
}

// A hostname is refused rather than probed. Resolving one would let an operator
// point the check at something other than what was published.
#[test]
fn a_hostname_is_not_treated_as_a_public_address() {
    assert!(!RoutableAddress::is_public("example.com"));
    assert!(!RoutableAddress::is_public(""));
}
