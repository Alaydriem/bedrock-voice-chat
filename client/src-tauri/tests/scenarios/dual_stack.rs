use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use common::net::NegotiationProbe;
use common::structs::reachability::ReachabilityOutcome;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// The load-bearing claim of the dual-stack work: **one socket serves both address
/// families**. A `[::]` bind is only useful if IPv4 peers still arrive, and
/// s2n-quic-platform clearing `IPV6_V6ONLY` is what makes that true — a behaviour of
/// the pinned dependency, not of our code, so it has to be observed rather than
/// assumed.
///
/// The Version Negotiation probe is the right instrument: it needs no certificate
/// and no SNI, so it can dial the same listener as a v4 literal and as a v6 literal
/// and compare, which a real mTLS client cannot do without cert SAN gymnastics.
///
/// Requires the server cdylib to be pre-built:
/// `cargo build -p bedrock-voice-chat-server` in `server/`
#[tokio::test(flavor = "multi_thread")]
async fn a_dual_stack_listener_answers_on_both_families() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json_dual_stack(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let over_v6 = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), server.quic_port());
    let over_v4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), server.quic_port());

    let v6_outcome = NegotiationProbe::probe(over_v6).await;
    let v4_outcome = NegotiationProbe::probe(over_v4).await;

    assert!(
        v6_outcome.answered(),
        "a [::] listener must answer over IPv6, got {v6_outcome:?}"
    );
    assert!(
        v4_outcome.answered(),
        "a [::] listener must also answer over IPv4 — if this fails the socket is \
         v6-only and every existing IPv4 player is cut off, got {v4_outcome:?}"
    );
}

/// The regression that would hurt most. Every current player is IPv4, and the
/// production default `listen` is now `::`, so an IPv4 client has to complete a real
/// mTLS QUIC session against a v6-bound listener — arriving v4-mapped — with audio
/// and channel join intact.
#[tokio::test(flavor = "multi_thread")]
async fn an_ipv4_client_holds_a_session_against_a_dual_stack_listener() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json_dual_stack(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let code = server.login_code("Alice");

    // A v4 literal resolves to exactly one address, so this pins the client to IPv4
    // regardless of what the probe would have preferred.
    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let client = ClientProc::spawn("Alice", &code, &url, "test-channel");

    client
        .await_connected(Duration::from_secs(20))
        .expect("an IPv4 client must still connect to a dual-stack listener");

    client.shutdown();
}

/// The path an IPv6-capable host actually takes: `localhost` resolves to both `::1`
/// and `127.0.0.1`, so the reachability probe runs over both, endorses IPv6 because
/// the dual-stack listener answers there, and `CandidatePlan` orders the v6
/// candidate first. Exercises probe → verdict → candidate ordering → `[::]:0` client
/// socket → handshake as one sequence, which no unit test can cover.
#[tokio::test(flavor = "multi_thread")]
async fn a_dual_family_host_holds_a_session_against_a_dual_stack_listener() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json_dual_stack(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let code = server.login_code("Alice");

    // `localhost` is the only name in the server cert's SAN list that resolves to
    // both families, so it is what puts a real v6 candidate in front of the client.
    let url = format!("https://localhost:{}", server.rocket_port());

    let client = ClientProc::spawn("Alice", &code, &url, "test-channel");

    client
        .await_connected(Duration::from_secs(20))
        .expect("a host with both families must connect to a dual-stack listener");

    client.shutdown();
}

/// An operator who pins `listen` to IPv4 must be unaffected by any of this. Guards
/// the branch every existing deployment takes until it changes config.
#[tokio::test(flavor = "multi_thread")]
async fn an_ipv4_only_listener_does_not_answer_over_ipv6() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let over_v4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), server.quic_port());
    let over_v6 = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), server.quic_port());

    assert!(
        NegotiationProbe::probe(over_v4).await.answered(),
        "a 127.0.0.1 listener must answer over IPv4"
    );
    assert_eq!(
        NegotiationProbe::probe(over_v6).await,
        ReachabilityOutcome::Silent,
        "a 127.0.0.1 listener has nothing on IPv6; reporting otherwise would mean \
         the probe cannot tell the two families apart"
    );
}
