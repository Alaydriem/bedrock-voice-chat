use common::structs::reachability::{
    AddressFamily, AddressFamilyPreference, AnsweredVia, EndpointReachability, ReachabilityOutcome,
    ServerReachability,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

fn v6(last: u16, port: u16) -> SocketAddr {
    SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, last)),
        port,
    )
}

fn v4(last: u8, port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, last)), port)
}

fn answered(rtt_micros: u32) -> ReachabilityOutcome {
    ReachabilityOutcome::Answered {
        via: AnsweredVia::VersionNegotiation,
        rtt_micros,
    }
}

#[test]
fn ipv4_mapped_address_is_classified_as_ipv4() {
    let mapped = IpAddr::V6(Ipv4Addr::new(203, 0, 113, 1).to_ipv6_mapped());

    assert_eq!(AddressFamily::of(&mapped), AddressFamily::Ipv4);
}

#[test]
fn one_answering_ipv6_endpoint_makes_ipv6_preferred() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v4(1, 443), answered(9_000), None),
            EndpointReachability::new(v6(1, 443), answered(40_000), None),
        ],
        Vec::new(),
    );

    assert_eq!(report.preference(), AddressFamilyPreference::PreferIpv6);
}

#[test]
fn unreachable_ipv6_endpoints_leave_ipv4_preferred() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v6(1, 443), ReachabilityOutcome::NoRoute, None),
            EndpointReachability::new(v6(2, 443), ReachabilityOutcome::Silent, None),
            EndpointReachability::new(v4(1, 443), answered(9_000), None),
        ],
        Vec::new(),
    );

    assert_eq!(report.preference(), AddressFamilyPreference::PreferIpv4);
}

// The HTTPS layer reports on a different transport and must not sway which
// family the QUIC candidate order prefers.
#[test]
fn an_answering_https_endpoint_does_not_make_ipv6_preferred() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![EndpointReachability::new(
            v6(1, 443),
            ReachabilityOutcome::NoRoute,
            None,
        )],
        vec![EndpointReachability::new(
            v6(1, 443),
            ReachabilityOutcome::Answered {
                via: AnsweredVia::Https,
                rtt_micros: 30_000,
            },
            None,
        )],
    );

    assert_eq!(report.preference(), AddressFamilyPreference::PreferIpv4);
}

#[test]
fn preference_orders_the_families_it_prefers_first() {
    assert_eq!(
        AddressFamilyPreference::PreferIpv6.order(),
        [AddressFamily::Ipv6, AddressFamily::Ipv4]
    );
    assert!(AddressFamilyPreference::PreferIpv6.is_preferred(AddressFamily::Ipv6));
    assert!(!AddressFamilyPreference::PreferIpv6.is_preferred(AddressFamily::Ipv4));
    assert!(AddressFamilyPreference::PreferIpv4.is_preferred(AddressFamily::Ipv4));
}

#[test]
fn measured_latency_is_retrievable_per_address_and_port() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v6(1, 443), answered(40_000), None),
            EndpointReachability::new(v6(1, 8443), answered(41_000), None),
            EndpointReachability::new(v6(2, 443), ReachabilityOutcome::Silent, None),
        ],
        Vec::new(),
    );

    assert_eq!(report.rtt_for(&v6(1, 443).ip(), 443), Some(40_000));
    assert_eq!(report.rtt_for(&v6(1, 8443).ip(), 8443), Some(41_000));
    assert_eq!(report.rtt_for(&v6(2, 443).ip(), 443), None);
}
