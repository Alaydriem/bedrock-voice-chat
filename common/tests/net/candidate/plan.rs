use common::net::{CandidatePlan, NetTimeouts};
use common::structs::reachability::{
    AddressFamily, AnsweredVia, EndpointReachability, ReachabilityOutcome, ServerReachability,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

fn v6(last: u16) -> IpAddr {
    IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, last))
}

fn v4(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(203, 0, 113, last))
}

fn answered(rtt_micros: u32) -> ReachabilityOutcome {
    ReachabilityOutcome::Answered {
        via: AnsweredVia::VersionNegotiation,
        rtt_micros,
    }
}

fn report(endpoints: Vec<EndpointReachability>) -> ServerReachability {
    ServerReachability::new("plan.test".to_string(), endpoints, Vec::new(), Vec::new())
}

fn ipv6_preferred() -> ServerReachability {
    report(vec![
        EndpointReachability::new(SocketAddr::new(v6(1), 443), answered(40_000), None),
        EndpointReachability::new(SocketAddr::new(v4(1), 443), answered(9_000), None),
    ])
}

fn ipv4_preferred() -> ServerReachability {
    report(vec![
        EndpointReachability::new(
            SocketAddr::new(v6(1), 443),
            ReachabilityOutcome::NoRoute,
            None,
        ),
        EndpointReachability::new(SocketAddr::new(v4(1), 443), answered(9_000), None),
    ])
}

// Family is the tiebreak inside a port, never the other way round. Here the measured
// port also leads, so this says nothing about port order on its own — the cases below
// cover that.
#[test]
fn candidates_are_ordered_by_port_then_family() {
    let plan = CandidatePlan::build(&[v6(1), v4(1)], &[443, 8443], &ipv6_preferred());

    let ports: Vec<u16> = plan.candidates().iter().map(|c| c.port()).collect();
    assert_eq!(ports, vec![443, 443, 8443, 8443]);

    let families: Vec<AddressFamily> = plan.candidates().iter().map(|c| c.family()).collect();
    assert_eq!(
        families,
        vec![
            AddressFamily::Ipv6,
            AddressFamily::Ipv4,
            AddressFamily::Ipv6,
            AddressFamily::Ipv4
        ]
    );
}

#[test]
fn an_ipv4_verdict_inverts_the_family_order_without_touching_port_order() {
    let plan = CandidatePlan::build(&[v6(1), v4(1)], &[443, 8443], &ipv4_preferred());

    let ports: Vec<u16> = plan.candidates().iter().map(|c| c.port()).collect();
    assert_eq!(ports, vec![443, 443, 8443, 8443]);

    let families: Vec<AddressFamily> = plan.candidates().iter().map(|c| c.family()).collect();
    assert_eq!(
        families,
        vec![
            AddressFamily::Ipv4,
            AddressFamily::Ipv6,
            AddressFamily::Ipv4,
            AddressFamily::Ipv6
        ]
    );
}

// This is the invariant the whole design rests on. A verdict that removed
// candidates could strand a player whose probe was wrong, which is the exact
// silent failure this work exists to end.
#[test]
fn a_negative_verdict_reorders_and_never_removes() {
    let addrs = [v6(1), v4(1)];
    let ports = [443, 8443];

    let preferring_v6 = CandidatePlan::build(&addrs, &ports, &ipv6_preferred());
    let preferring_v4 = CandidatePlan::build(&addrs, &ports, &ipv4_preferred());

    assert_eq!(preferring_v6.candidates().len(), 4);
    assert_eq!(preferring_v4.candidates().len(), 4);
}

// s2n-quic writes a bare sockaddr_in for an IPv4 destination, which a v6 socket
// rejects outright. A dual-stack socket must therefore be handed the v4-mapped
// form.
#[test]
fn ipv4_candidates_are_v4_mapped_when_the_socket_is_ipv6() {
    let plan = CandidatePlan::build(&[v6(1), v4(1)], &[443], &ipv6_preferred());

    assert!(plan.requires_v6_socket());

    let dialed: Vec<SocketAddr> = plan.candidates().iter().map(|c| c.dial()).collect();
    let mapped = SocketAddr::new(IpAddr::V6(Ipv4Addr::new(203, 0, 113, 1).to_ipv6_mapped()), 443);

    assert!(dialed.contains(&mapped));
    assert!(!dialed.contains(&SocketAddr::new(v4(1), 443)));
}

// A client with nothing to try over v6 must keep behaving exactly as it does
// today, on a plain v4 socket with unmapped addresses.
#[test]
fn a_plan_without_ipv6_addresses_stays_on_a_plain_ipv4_socket() {
    let plan = CandidatePlan::build(&[v4(1)], &[443], &ipv4_preferred());

    assert!(!plan.requires_v6_socket());
    assert_eq!(plan.candidates()[0].dial(), SocketAddr::new(v4(1), 443));
}

#[test]
fn measured_latency_orders_addresses_within_a_family() {
    let slow = v6(1);
    let fast = v6(2);
    let measured = report(vec![
        EndpointReachability::new(SocketAddr::new(slow, 443), answered(90_000), None),
        EndpointReachability::new(SocketAddr::new(fast, 443), answered(20_000), None),
    ]);

    let plan = CandidatePlan::build(&[slow, fast], &[443], &measured);
    let dialed: Vec<IpAddr> = plan.candidates().iter().map(|c| c.dial().ip()).collect();

    assert_eq!(dialed, vec![fast, slow]);
}

// The probe's verdict decides the order and nothing else. A shorter budget for the
// fallback family gave the least time to the attempt made after the preferred family had
// already failed — the one most likely to be the unusual path that works.
#[test]
fn every_family_gets_the_same_attempt_budget() {
    let plan = CandidatePlan::build(&[v6(1), v4(1)], &[443], &ipv6_preferred());

    for candidate in plan.candidates() {
        assert_eq!(candidate.budget(), NetTimeouts::HANDSHAKE);
    }
}

// The rebind path after a failed [::] bind: v6 candidates become undialable, so
// they go, and what remains must be unmapped for a plain v4 socket.
#[test]
fn dropping_ipv6_leaves_unmapped_ipv4_candidates() {
    let plan = CandidatePlan::build(&[v6(1), v4(1)], &[443], &ipv6_preferred()).without_ipv6();

    assert!(!plan.requires_v6_socket());
    assert_eq!(plan.candidates().len(), 1);
    assert_eq!(plan.candidates()[0].dial(), SocketAddr::new(v4(1), 443));
}

#[test]
fn a_plan_with_no_addresses_is_empty() {
    let plan = CandidatePlan::build(&[], &[443], &ipv4_preferred());

    assert!(plan.is_empty());
}

// The defect this ordering exists to end. A server advertising a port that nothing
// answers on, ahead of one that answers instantly, spent a full handshake budget on the
// dead one before reaching the live one — after the probe had already measured both.
#[test]
fn a_port_that_did_not_answer_sorts_below_one_that_did() {
    let addr = v4(1);
    let measured = report(vec![
        EndpointReachability::new(SocketAddr::new(addr, 28280), ReachabilityOutcome::Silent, None),
        EndpointReachability::new(SocketAddr::new(addr, 443), answered(16_000), None),
    ]);

    let plan = CandidatePlan::build(&[addr], &[28280, 443], &measured);
    let ports: Vec<u16> = plan.candidates().iter().map(|c| c.port()).collect();

    assert_eq!(ports, vec![443, 28280]);
}

#[test]
fn the_faster_measured_port_leads_the_walk() {
    let addr = v4(1);
    let measured = report(vec![
        EndpointReachability::new(SocketAddr::new(addr, 443), answered(90_000), None),
        EndpointReachability::new(SocketAddr::new(addr, 8443), answered(12_000), None),
    ]);

    let plan = CandidatePlan::build(&[addr], &[443, 8443], &measured);
    let ports: Vec<u16> = plan.candidates().iter().map(|c| c.port()).collect();

    assert_eq!(ports, vec![8443, 443]);
}

// With nothing measured to separate them the operator's order is the only statement
// available about which way in they intend, so it still decides.
#[test]
fn the_operator_order_survives_when_no_port_was_measured() {
    let addr = v4(1);
    let measured = report(vec![EndpointReachability::new(
        SocketAddr::new(addr, 443),
        ReachabilityOutcome::Silent,
        None,
    )]);

    let plan = CandidatePlan::build(&[addr], &[8443, 443], &measured);
    let ports: Vec<u16> = plan.candidates().iter().map(|c| c.port()).collect();

    assert_eq!(ports, vec![8443, 443]);
}

// Ordering is the only thing a measurement may change. A silent port is still dialled,
// because a probe that was wrong must cost time and never connectivity.
#[test]
fn a_port_that_did_not_answer_is_reordered_and_never_removed() {
    let addr = v4(1);
    let measured = report(vec![
        EndpointReachability::new(SocketAddr::new(addr, 28280), ReachabilityOutcome::Silent, None),
        EndpointReachability::new(SocketAddr::new(addr, 443), answered(16_000), None),
    ]);

    let plan = CandidatePlan::build(&[addr], &[28280, 443], &measured);

    assert_eq!(plan.candidates().len(), 2);
}

// A port is ranked by the best answer any address gave on it, so one dead address does
// not sink a port that another address reaches instantly.
#[test]
fn a_port_is_ranked_by_its_fastest_answering_address() {
    let slow = v4(1);
    let fast = v4(2);
    let measured = report(vec![
        EndpointReachability::new(SocketAddr::new(slow, 8443), ReachabilityOutcome::Silent, None),
        EndpointReachability::new(SocketAddr::new(fast, 8443), answered(9_000), None),
        EndpointReachability::new(SocketAddr::new(slow, 443), answered(80_000), None),
        EndpointReachability::new(SocketAddr::new(fast, 443), answered(85_000), None),
    ]);

    let plan = CandidatePlan::build(&[slow, fast], &[443, 8443], &measured);
    let ports: Vec<u16> = plan.candidates().iter().map(|c| c.port()).collect();

    assert_eq!(ports, vec![8443, 8443, 443, 443]);
}
