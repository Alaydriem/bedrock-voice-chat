use bvc_server_lib::relay::peer::dial::PeerDialer;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

fn v6(last: u16) -> IpAddr {
    IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, last))
}

fn v4(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(203, 0, 113, last))
}

// A peer that publishes only AAAA is unreachable from a v4 socket. The relay is the
// one path where both ends are servers we operate, so the bind choice is entirely
// ours to get right.
#[test]
fn a_peer_with_an_ipv6_address_requires_a_dual_stack_socket() {
    assert_eq!(PeerDialer::bind_address(&[v6(1)]), "[::]:0");
    assert_eq!(PeerDialer::bind_address(&[v4(1), v6(1)]), "[::]:0");
}

#[test]
fn an_ipv4_only_peer_keeps_the_plain_ipv4_socket() {
    assert_eq!(PeerDialer::bind_address(&[v4(1)]), "0.0.0.0:0");
    assert_eq!(PeerDialer::bind_address(&[]), "0.0.0.0:0");
}

// A v4-mapped address is an IPv4 destination, so it must not drag the socket onto
// the dual-stack path by itself.
#[test]
fn a_v4_mapped_address_does_not_require_a_dual_stack_socket() {
    let mapped = IpAddr::V6(Ipv4Addr::new(203, 0, 113, 1).to_ipv6_mapped());

    assert_eq!(PeerDialer::bind_address(&[mapped]), "0.0.0.0:0");
}

// s2n-quic writes a bare sockaddr_in for an IPv4 destination and does no conversion
// of its own, so a dual-stack socket has to be handed the v4-mapped form or the
// send fails with an address-family error.
#[test]
fn ipv4_destinations_are_v4_mapped_for_a_dual_stack_socket() {
    let addrs = [v4(1), v6(1)];
    let bind = PeerDialer::bind_address(&addrs);

    let dialed = PeerDialer::dial_targets(&addrs, 8443, bind);

    let mapped = SocketAddr::new(IpAddr::V6(Ipv4Addr::new(203, 0, 113, 1).to_ipv6_mapped()), 8443);
    assert!(dialed.contains(&mapped));
    assert!(!dialed.contains(&SocketAddr::new(v4(1), 8443)));
}

#[test]
fn ipv4_destinations_stay_unmapped_on_a_plain_ipv4_socket() {
    let addrs = [v4(1)];
    let bind = PeerDialer::bind_address(&addrs);

    let dialed = PeerDialer::dial_targets(&addrs, 8443, bind);

    assert_eq!(dialed, vec![SocketAddr::new(v4(1), 8443)]);
}

// Every resolved address stays a candidate. Dropping one because of a guess about
// which family works would strand a peer whose only address is the dropped one.
#[test]
fn every_resolved_address_becomes_a_candidate() {
    let addrs = [v4(1), v4(2), v6(1)];
    let bind = PeerDialer::bind_address(&addrs);

    assert_eq!(PeerDialer::dial_targets(&addrs, 8443, bind).len(), 3);
}
