use common::net::RouteProbe;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

// A loopback destination always resolves to a loopback source on every platform,
// which makes it the one deterministic route assertion available.
#[test]
fn a_loopback_destination_selects_a_loopback_source() {
    let dest = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 443);

    assert_eq!(
        RouteProbe::source_for(dest),
        Some(IpAddr::V4(Ipv4Addr::LOCALHOST))
    );
}

// An embedded BVC server is reached over loopback, so a loopback destination has
// to count as routable. Judging the source in isolation would report a local
// deployment as unreachable.
#[test]
fn a_loopback_destination_is_routable() {
    let dest = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 443);

    assert!(RouteProbe::is_routable(dest));
}

// Reaching a *public* server over a loopback source is not connectivity. This is
// the pairing the gate exists to reject, and it cannot be produced by a real
// routing table on demand, so the rule is asserted directly.
#[test]
fn a_loopback_source_cannot_serve_a_global_destination() {
    let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let global = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1));

    assert!(!RouteProbe::source_suits(&loopback, &global));
    assert!(RouteProbe::source_suits(&loopback, &loopback));
}

// The pairing that describes the Thailand player's host: a CLAT-synthesised or
// link-local source reaching for a global IPv6 address means there is no native
// v6 path, however well the socket bound.
#[test]
fn a_broken_ipv6_source_cannot_serve_a_global_ipv6_destination() {
    let global = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
    let mapped = IpAddr::V6(Ipv4Addr::new(203, 0, 113, 1).to_ipv6_mapped());
    let link_local = IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));

    assert!(!RouteProbe::source_suits(&mapped, &global));
    assert!(!RouteProbe::source_suits(&link_local, &global));
    assert!(RouteProbe::source_suits(&global, &global));
}

#[test]
fn private_ipv4_sources_are_usable_because_nat_is_normal() {
    assert!(RouteProbe::is_global_unicast(&IpAddr::V4(Ipv4Addr::new(
        192, 168, 1, 20
    ))));
    assert!(RouteProbe::is_global_unicast(&IpAddr::V4(Ipv4Addr::new(
        100, 64, 0, 7
    ))));
}

#[test]
fn unusable_ipv4_sources_are_rejected() {
    assert!(!RouteProbe::is_global_unicast(&IpAddr::V4(
        Ipv4Addr::UNSPECIFIED
    )));
    assert!(!RouteProbe::is_global_unicast(&IpAddr::V4(
        Ipv4Addr::LOCALHOST
    )));
    assert!(!RouteProbe::is_global_unicast(&IpAddr::V4(Ipv4Addr::new(
        169, 254, 3, 4
    ))));
}

#[test]
fn a_global_ipv6_source_is_usable() {
    let global = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);

    assert!(RouteProbe::is_global_unicast(&IpAddr::V6(global)));
}

// A CLAT-synthesised v4 source, a link-local address, and a unique-local address
// all mean the host cannot reach a public v6 server natively.
#[test]
fn unusable_ipv6_sources_are_rejected() {
    let mapped = Ipv4Addr::new(203, 0, 113, 1).to_ipv6_mapped();
    let link_local = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let unique_local = Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1);

    assert!(!RouteProbe::is_global_unicast(&IpAddr::V6(
        Ipv6Addr::UNSPECIFIED
    )));
    assert!(!RouteProbe::is_global_unicast(&IpAddr::V6(
        Ipv6Addr::LOCALHOST
    )));
    assert!(!RouteProbe::is_global_unicast(&IpAddr::V6(mapped)));
    assert!(!RouteProbe::is_global_unicast(&IpAddr::V6(link_local)));
    assert!(!RouteProbe::is_global_unicast(&IpAddr::V6(unique_local)));
}
