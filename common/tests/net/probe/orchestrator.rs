use super::negotiation::{spawn_negotiating_server, spawn_silent_server};
use common::net::{NetTimeouts, ReachabilityProbe};
use common::structs::reachability::{
    AddressFamilyPreference, ReachabilityRequest, ReachabilityVerdict,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};

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
        false,
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
        false,
    )
}

fn request_with_fallback(voice_websocket: bool) -> ReachabilityRequest {
    ReachabilityRequest::new(
        "probe.test".to_string(),
        vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        vec![1u16],
        "https://127.0.0.1:1/api/config".to_string(),
        1,
        voice_websocket,
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

// A server that never claimed the fallback transport is not probed for one. The leg is
// empty rather than silent, which is the difference between "not offered" and "offered
// and did not answer".
#[tokio::test]
async fn an_unadvertised_fallback_transport_is_not_probed() {
    let probe = ReachabilityProbe::new();

    let report = probe.evaluate(&request_with_fallback(false)).await;

    assert!(report.ws().is_empty());
}

#[tokio::test]
async fn an_advertised_fallback_transport_is_probed_per_address() {
    let probe = ReachabilityProbe::new();
    let request = request_with_fallback(true);

    let report = probe.evaluate(&request).await;

    assert_eq!(report.ws().len(), request.addrs.len());
}

// The same trap the port set has. A report taken before the capability was known carries
// no fallback leg, and answering with it would report no fallback path on a server that
// has one.
#[tokio::test]
async fn a_report_cached_without_the_fallback_leg_is_remeasured_when_one_is_asked_for() {
    let probe = ReachabilityProbe::new();

    let unaware = probe.evaluate(&request_with_fallback(false)).await;
    assert!(unaware.ws().is_empty());

    let aware = probe.evaluate(&request_with_fallback(true)).await;

    assert!(!aware.ws().is_empty());
}

// The wider report already measured the leg, so a caller that does not care about it must
// not pay for a second measurement.
#[tokio::test]
async fn a_report_cached_with_the_fallback_leg_still_answers_a_request_without_one() {
    let probe = ReachabilityProbe::new();

    probe.evaluate(&request_with_fallback(true)).await;
    let unaware = probe.evaluate(&request_with_fallback(false)).await;

    assert!(!unaware.ws().is_empty());
}

fn request_for_ports(ports: Vec<u16>) -> ReachabilityRequest {
    ReachabilityRequest::new(
        "probe.test".to_string(),
        vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        ports,
        "https://127.0.0.1:1/api/config".to_string(),
        1,
        false,
    )
}

/**
The login field's whole complaint. One endpoint answers at once and another is blackholed, so
the complete measurement cannot finish until the silent one has spent a negotiation budget and
then a handshake budget on top of it. An address field that waited for that reported a 10 ms
round trip fifteen seconds after the fact.
*/
#[tokio::test]
async fn a_voice_path_answers_without_waiting_for_a_blackholed_endpoint() {
    let answering = spawn_negotiating_server().await;
    let blackholed = spawn_silent_server().await;
    let request = request_for_ports(vec![answering.port(), blackholed.port()]);

    let probe = ReachabilityProbe::new_shared();
    let started = Instant::now();
    let report = probe.evaluate_any_voice_path(&request).await;
    let elapsed = started.elapsed();

    assert_eq!(report.verdict(), ReachabilityVerdict::Ready);
    assert!(
        elapsed < NetTimeouts::NEGOTIATION,
        "answered in {elapsed:?}, which is not ahead of the {:?} the silent endpoint alone spends",
        NetTimeouts::NEGOTIATION
    );
}

// The complete measurement is what the connect path reads, and an early return must not cost
// it. The report handed back early may be missing legs; the cached one may not.
#[tokio::test]
async fn the_measurement_behind_an_early_answer_still_caches_a_complete_report() {
    let first = spawn_negotiating_server().await;
    let second = spawn_negotiating_server().await;
    let request = request_for_ports(vec![first.port(), second.port()]);

    let probe = ReachabilityProbe::new_shared();
    probe.evaluate_any_voice_path(&request).await;

    // The background task outlives the answer, so the cache fills a moment after it.
    for _ in 0..100 {
        if probe.is_cached(&request.host) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let cached = probe.evaluate(&request).await;
    for port in [first.port(), second.port()] {
        assert!(
            cached.quic().iter().any(|e| e.port() == port),
            "the cached report is missing udp/{port}"
        );
    }
}

// A host with nothing reachable has no early answer to give, so this must behave exactly like
// the complete measurement rather than inventing a verdict to return sooner.
#[tokio::test]
async fn nothing_reachable_yields_the_same_verdict_as_the_complete_measurement() {
    let request = closed_loopback_request();

    let early = ReachabilityProbe::new_shared()
        .evaluate_any_voice_path(&request)
        .await;
    let complete = ReachabilityProbe::new().evaluate(&request).await;

    assert_eq!(early.verdict(), complete.verdict());
    assert!(!early.any_voice_path());
}

// Ports are the request's, so a probe measuring one and answering about another would be
// asserting something about an endpoint nobody asked for.
#[tokio::test]
async fn an_early_answer_names_the_host_it_measured() {
    let answering = spawn_negotiating_server().await;
    let request = request_for_ports(vec![answering.port()]);

    let report = ReachabilityProbe::new_shared()
        .evaluate_any_voice_path(&request)
        .await;

    assert_eq!(report.host(), "probe.test");
}

// A cached report is complete and already answers the narrower question, so the early path
// must hand it straight back rather than starting a measurement beside it.
#[tokio::test]
async fn a_cached_report_answers_the_early_path_too() {
    let answering = spawn_negotiating_server().await;
    let blackholed = spawn_silent_server().await;
    let ports = vec![answering.port(), blackholed.port()];

    let probe = ReachabilityProbe::new_shared();
    probe.evaluate(&request_for_ports(ports.clone())).await;

    let report = probe
        .evaluate_any_voice_path(&request_for_ports(ports.clone()))
        .await;

    assert_eq!(report.verdict(), ReachabilityVerdict::Ready);
    for port in ports {
        assert!(
            report.quic().iter().any(|e| e.port() == port),
            "the cached report handed back early is missing udp/{port}"
        );
    }
}
