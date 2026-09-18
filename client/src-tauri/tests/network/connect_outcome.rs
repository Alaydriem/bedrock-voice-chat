//! What one connect walk reports about the transport that carried it.
//!
//! The behaviour under test is not "are the fields present" but the one question the event
//! exists to answer: which transport carried this session. A player whose network blocks UDP
//! reaches the server over the WebSocket fallback, and an outcome that describes only the QUIC
//! walk records that session as a failure — the walk did fail, and the player is connected.
//! Every case below is a session that worked or did not, judged by the transport, not the walk.

use bvc_client_lib::network::{AttemptResult, ConnectOutcome, FallbackReason};
use common::net::ConnectCandidate;
use common::structs::metrics::TransportKind;
use common::structs::reachability::AddressFamily;
use std::net::SocketAddr;
use std::time::Duration;

const SERVER: &str = "https://voice.example.com";

fn candidate(port: u16, family: AddressFamily) -> ConnectCandidate {
    let dial: SocketAddr = match family {
        AddressFamily::Ipv6 => format!("[::1]:{port}").parse().unwrap(),
        AddressFamily::Ipv4 => format!("127.0.0.1:{port}").parse().unwrap(),
    };
    ConnectCandidate::new(dial, family, port, Duration::from_secs(1))
}

fn property(outcome: &ConnectOutcome, key: &str) -> serde_json::Value {
    outcome
        .properties(SERVER)
        .properties
        .get(key)
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

#[test]
fn a_websocket_session_reports_the_websocket_transport() {
    let mut outcome = ConnectOutcome::new();
    outcome.carried(TransportKind::WebSocket);

    assert_eq!(property(&outcome, "transport"), "websocket");
    assert_eq!(property(&outcome, "connected"), true);
}

#[test]
fn a_quic_session_reports_the_quic_transport_and_its_winning_candidate() {
    let mut outcome = ConnectOutcome::new();
    outcome.record(candidate(443, AddressFamily::Ipv6), AttemptResult::TimedOut);
    outcome.record(candidate(443, AddressFamily::Ipv4), AttemptResult::Connected);
    outcome.carried(TransportKind::Quic);

    assert_eq!(property(&outcome, "transport"), "quic");
    assert_eq!(property(&outcome, "connected"), true);
    assert_eq!(property(&outcome, "winning_port"), 443);
    assert_eq!(property(&outcome, "winning_family"), "Ipv4");
    assert_eq!(
        property(&outcome, "walk"),
        "443/Ipv6=timed_out,443/Ipv4=connected"
    );
}

#[test]
fn a_quic_walk_that_failed_into_websocket_reports_a_connected_session() {
    // The case the old shape got wrong: every candidate timed out, and the player is
    // nonetheless talking. Reporting this as `connected: false` undercounts exactly the
    // players the fallback exists for.
    let mut outcome = ConnectOutcome::new();
    outcome.record(candidate(443, AddressFamily::Ipv6), AttemptResult::TimedOut);
    outcome.record(candidate(443, AddressFamily::Ipv4), AttemptResult::TimedOut);
    outcome.fell_back(FallbackReason::QuicFailed);
    outcome.carried(TransportKind::WebSocket);

    assert_eq!(property(&outcome, "transport"), "websocket");
    assert_eq!(property(&outcome, "connected"), true);
    assert_eq!(property(&outcome, "fallback"), "quic_failed");
    assert_eq!(property(&outcome, "timed_out"), 2);
    assert_eq!(
        property(&outcome, "walk"),
        "443/Ipv6=timed_out,443/Ipv4=timed_out",
        "the walk that failed is still the evidence for why this session is on WebSocket"
    );
    assert_eq!(
        property(&outcome, "winning_port"),
        serde_json::Value::Null,
        "no QUIC candidate won, so none may be named"
    );
}

#[test]
fn a_demoted_host_reports_a_websocket_session_that_walked_nothing() {
    // A demoted host skips the walk entirely. Emitting nothing here is what made every
    // demoted session invisible; an empty walk with a named reason is the distinguishing
    // record.
    let mut outcome = ConnectOutcome::new();
    outcome.fell_back(FallbackReason::Demoted);
    outcome.carried(TransportKind::WebSocket);

    assert_eq!(property(&outcome, "transport"), "websocket");
    assert_eq!(property(&outcome, "connected"), true);
    assert_eq!(property(&outcome, "fallback"), "demoted");
    assert_eq!(property(&outcome, "attempts"), 0);
    assert_eq!(property(&outcome, "walk"), "");
}

#[test]
fn a_connect_that_carried_nothing_reports_no_transport() {
    let mut outcome = ConnectOutcome::new();
    outcome.record(candidate(443, AddressFamily::Ipv4), AttemptResult::Rejected);
    outcome.fell_back(FallbackReason::QuicFailed);

    assert_eq!(property(&outcome, "transport"), "none");
    assert_eq!(property(&outcome, "connected"), false);
    assert_eq!(property(&outcome, "rejected"), 1);
}

#[test]
fn a_probe_that_chose_nothing_still_reports_an_outcome() {
    // No transport answered the probe, so no dial happens at all. The event still has to
    // fire: a network on which nothing is reachable is the outcome most worth counting.
    let outcome = ConnectOutcome::new();

    assert_eq!(property(&outcome, "transport"), "none");
    assert_eq!(property(&outcome, "connected"), false);
    assert_eq!(property(&outcome, "attempts"), 0);
    assert_eq!(property(&outcome, "fallback"), "none");
    assert_eq!(property(&outcome, "server"), SERVER);
}

#[test]
fn a_websocket_first_choice_that_fell_to_quic_names_that_direction() {
    // The probe's verdict can be wrong in either direction. A WebSocket choice that did not
    // carry, rescued by the QUIC walk, must be separable from the common direction — it is
    // the signal that the probe itself is misreading this network.
    let mut outcome = ConnectOutcome::new();
    outcome.fell_back(FallbackReason::WebSocketFailed);
    outcome.record(candidate(443, AddressFamily::Ipv4), AttemptResult::Connected);
    outcome.carried(TransportKind::Quic);

    assert_eq!(property(&outcome, "fallback"), "websocket_failed");
    assert_eq!(property(&outcome, "transport"), "quic");
}
