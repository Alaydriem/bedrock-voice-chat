use common::net::ReachabilityProbe;
use common::structs::reachability::{AddressFamilyPreference, ReachabilityRequest};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// Port 1 on loopback is closed, so every layer runs to exhaustion without any
// endpoint answering. That is the shape of a host with nothing reachable.
fn closed_loopback_request() -> ReachabilityRequest {
    ReachabilityRequest::new(
        "probe.test".to_string(),
        vec![
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        ],
        vec![1u16],
        "https://127.0.0.1:1/api/config".to_string(),
        1,
    )
}

#[tokio::test]
async fn an_evaluated_host_is_cached_and_invalidation_clears_it() {
    let probe = ReachabilityProbe::new();
    let request = closed_loopback_request();

    assert!(!probe.is_cached(&request.host));

    probe.evaluate(&request).await;

    assert!(probe.is_cached(&request.host));

    probe.invalidate(&request.host).await;

    assert!(!probe.is_cached(&request.host));
}

// Nothing answered, so the verdict has to fall back rather than sit undecided.
#[tokio::test]
async fn a_host_with_no_answering_ipv6_endpoint_prefers_ipv4() {
    let probe = ReachabilityProbe::new();
    let request = closed_loopback_request();

    assert_eq!(
        probe.preference(&request).await,
        AddressFamilyPreference::PreferIpv4
    );
}

fn request_with_ports(quic_ports: Vec<u16>) -> ReachabilityRequest {
    ReachabilityRequest::new(
        "probe.test".to_string(),
        vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        quic_ports,
        "https://127.0.0.1:1/api/config".to_string(),
        1,
    )
}

// A preflight measures only the ports it knew about. Connect later knows more, and
// handing it a cached report that never touched the extra port would have it assert
// something about an endpoint nobody probed.
#[tokio::test]
async fn a_report_cached_for_fewer_ports_is_remeasured_when_more_are_asked_for() {
    let probe = ReachabilityProbe::new();

    let preflight = probe.evaluate(&request_with_ports(vec![1u16])).await;
    assert!(!preflight.quic().iter().any(|e| e.port() == 2));

    let connect = probe.evaluate(&request_with_ports(vec![1u16, 2u16])).await;

    assert!(connect.quic().iter().any(|e| e.port() == 2));
}

// The wider report already answers the narrower question, so asking it again must
// not pay for a second measurement.
#[tokio::test]
async fn a_report_cached_for_more_ports_still_answers_a_narrower_request() {
    let probe = ReachabilityProbe::new();

    probe.evaluate(&request_with_ports(vec![1u16, 2u16])).await;
    let narrower = probe.evaluate(&request_with_ports(vec![1u16])).await;

    assert!(narrower.quic().iter().any(|e| e.port() == 2));
}

#[tokio::test]
async fn every_requested_address_and_port_appears_in_the_report() {
    let probe = ReachabilityProbe::new();
    let request = closed_loopback_request();

    let report = probe.evaluate(&request).await;

    assert_eq!(
        report.quic().len(),
        request.addrs.len() * request.quic_ports.len()
    );
    assert_eq!(report.https().len(), request.addrs.len());
    assert_eq!(report.host(), "probe.test");
}
