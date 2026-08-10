use common::structs::metrics::TransportKind;
use bvc_client_lib::diagnostics::LinkSession;
use common::structs::reachability::AddressFamily;

const CA_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIB...
-----END CERTIFICATE-----";

#[test]
fn the_recorded_family_and_port_are_reported_unchanged() {
    // This pins storage only. The invariant that matters — that the family comes from the winning
    // connect candidate rather than from its dial address, which is v4-mapped on a dual-stack
    // socket — lives at the call site in `network/stream/mod.rs` and is not testable from here
    // without a live handshake. `scenarios::dual_stack` covers that end.
    let session = LinkSession::new();
    session.set(Some(AddressFamily::Ipv4), 443, TransportKind::Quic, "bvc.example.com".to_string(), CA_PEM);

    assert_eq!(session.family(), Some(AddressFamily::Ipv4));
    assert_eq!(session.port(), Some(443));
}

#[test]
fn an_ipv6_session_reports_ipv6() {
    let session = LinkSession::new();
    session.set(Some(AddressFamily::Ipv6), 443, TransportKind::Quic, "bvc.example.com".to_string(), CA_PEM);

    assert_eq!(session.family(), Some(AddressFamily::Ipv6));
}

#[test]
fn uptime_is_zero_before_a_connection_is_recorded() {
    let session = LinkSession::new();

    assert!(!session.is_connected());
    assert_eq!(session.uptime_secs(), 0);
    assert_eq!(session.family(), None);
    assert_eq!(session.port(), None);
}

#[test]
fn clearing_the_session_resets_port_family_and_uptime() {
    let session = LinkSession::new();
    session.set(Some(AddressFamily::Ipv6), 4443, TransportKind::Quic, "bvc.example.com".to_string(), CA_PEM);
    assert!(session.is_connected());

    session.clear();

    // A stopped stream must not keep reporting a stale port or a climbing uptime.
    assert!(!session.is_connected());
    assert_eq!(session.family(), None);
    assert_eq!(session.port(), None);
    assert_eq!(session.server(), None);
    assert_eq!(session.server_id(), None);
    assert_eq!(session.uptime_secs(), 0);
}

#[test]
fn reconnecting_replaces_the_previous_session() {
    let session = LinkSession::new();
    session.set(Some(AddressFamily::Ipv4), 443, TransportKind::Quic, "old.example.com".to_string(), CA_PEM);
    session.set(Some(AddressFamily::Ipv6), 4443, TransportKind::Quic, "new.example.com".to_string(), CA_PEM);

    assert_eq!(session.family(), Some(AddressFamily::Ipv6));
    assert_eq!(session.port(), Some(4443));
    assert_eq!(session.server(), Some("new.example.com".to_string()));
}
