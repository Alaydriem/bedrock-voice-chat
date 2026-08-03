use common::structs::reachability::{
    AddressFamily, AddressFamilyPreference, AnsweredVia, EndpointReachability, ReachabilityOutcome,
    ReachabilityVerdict, ServerReachability,
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

#[test]
fn best_quic_picks_the_lowest_rtt_answer() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v4(1, 443), answered(41_000), None),
            EndpointReachability::new(v4(1, 8443), answered(12_000), None),
        ],
        Vec::new(),
    );

    assert_eq!(report.best_quic().map(|e| e.port()), Some(8443));
}

// A silent endpoint measures nothing, so it must never win on ordering.
#[test]
fn best_quic_ignores_endpoints_that_did_not_answer() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v4(1, 443), ReachabilityOutcome::Silent, None),
            EndpointReachability::new(v4(1, 8443), answered(90_000), None),
        ],
        Vec::new(),
    );

    assert_eq!(report.best_quic().map(|e| e.port()), Some(8443));
}

#[test]
fn best_quic_is_none_when_nothing_answered() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v4(1, 443), ReachabilityOutcome::NoRoute, None),
            EndpointReachability::new(v4(1, 8443), ReachabilityOutcome::Silent, None),
        ],
        Vec::new(),
    );

    assert!(report.best_quic().is_none());
}

// HTTPS answering says the host is alive; it says nothing about the transport
// that carries voice, and a caption must not conflate them.
#[test]
fn best_quic_never_reports_an_https_endpoint() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        Vec::new(),
        vec![EndpointReachability::new(
            v4(1, 443),
            ReachabilityOutcome::Answered {
                via: AnsweredVia::Https,
                rtt_micros: 8_000,
            },
            None,
        )],
    );

    assert!(report.best_quic().is_none());
}

fn https_answered() -> Vec<EndpointReachability> {
    vec![EndpointReachability::new(
        v4(1, 443),
        ReachabilityOutcome::Answered {
            via: AnsweredVia::Https,
            rtt_micros: 8_000,
        },
        None,
    )]
}

#[test]
fn a_voice_path_that_answered_is_ready() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![EndpointReachability::new(v4(1, 443), answered(41_000), None)],
        https_answered(),
    );

    assert_eq!(report.verdict(), ReachabilityVerdict::Ready);
    assert!(report.any_quic_answered());
    assert_eq!(report.best_rtt_micros(), Some(41_000));
}

// The common, specific failure: a network that permits HTTPS and drops UDP. Telling
// someone "cannot reach this server" here would send them to check the wrong thing.
#[test]
fn https_answering_while_quic_is_silent_means_the_voice_path_is_blocked() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![EndpointReachability::new(
            v4(1, 443),
            ReachabilityOutcome::Silent,
            None,
        )],
        https_answered(),
    );

    assert_eq!(report.verdict(), ReachabilityVerdict::VoiceBlocked);
    assert!(!report.any_quic_answered());
    assert_eq!(report.best_rtt_micros(), None);
}

#[test]
fn nothing_answering_on_either_transport_is_unreachable() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![EndpointReachability::new(
            v4(1, 443),
            ReachabilityOutcome::Silent,
            None,
        )],
        vec![EndpointReachability::new(
            v4(1, 443),
            ReachabilityOutcome::Silent,
            None,
        )],
    );

    assert_eq!(report.verdict(), ReachabilityVerdict::Unreachable);
}

// No route is the local stack's answer, not the server's, and it earns its own
// message: nothing about the destination has been learned.
#[test]
fn no_route_on_every_quic_endpoint_is_its_own_verdict() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v4(1, 443), ReachabilityOutcome::NoRoute, None),
            EndpointReachability::new(v4(1, 8443), ReachabilityOutcome::NoRoute, None),
        ],
        https_answered(),
    );

    assert_eq!(report.verdict(), ReachabilityVerdict::NoRoute);
}

// A mix is not a routing problem: something did get out and stayed unanswered.
#[test]
fn a_mix_of_no_route_and_silence_is_not_reported_as_no_route() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v4(1, 443), ReachabilityOutcome::NoRoute, None),
            EndpointReachability::new(v4(1, 8443), ReachabilityOutcome::Silent, None),
        ],
        https_answered(),
    );

    assert_eq!(report.verdict(), ReachabilityVerdict::VoiceBlocked);
}

#[test]
fn best_rtt_reports_the_fastest_answering_endpoint() {
    let report = ServerReachability::new(
        "example.test".to_string(),
        vec![
            EndpointReachability::new(v4(1, 443), answered(41_000), None),
            EndpointReachability::new(v4(1, 8443), answered(12_000), None),
        ],
        Vec::new(),
    );

    assert_eq!(report.best_rtt_micros(), Some(12_000));
}
