use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use common::net::NegotiationProbe;
use common::structs::reachability::{AnsweredVia, ReachabilityOutcome};

use crate::harness::server::EmbeddedServer;

/// Validates the cert-free QUIC liveness probe against a REAL s2n-quic server.
///
/// The unit tests for `ProbeInitialPacket` assert the packet's shape, and the unit
/// tests for `NegotiationProbe` use a synthetic responder — neither can tell whether
/// s2n-quic decodes our hand-rolled Initial well enough to reach its version check
/// and answer. If it does not, the probe silently degrades to always reporting
/// `Silent`: the handshake layer still carries reachability, so nothing breaks and
/// every other test still passes, leaving a dead feature that looks alive. This test
/// is the only thing standing between that and a false sense of coverage.
///
/// Requires the server cdylib to be pre-built:
/// `cargo build -p bedrock-voice-chat-server` in `server/`
#[tokio::test(flavor = "multi_thread")]
async fn a_real_quic_server_answers_the_version_negotiation_probe() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let dest = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), server.quic_port());

    match NegotiationProbe::probe(dest).await {
        ReachabilityOutcome::Answered { via, rtt_micros } => {
            assert_eq!(
                via,
                AnsweredVia::VersionNegotiation,
                "a live QUIC server must answer via version negotiation"
            );
            assert!(
                (rtt_micros as u128) <= NegotiationProbe::BUDGET.as_micros(),
                "measured {rtt_micros}us against a {}us budget",
                NegotiationProbe::BUDGET.as_micros()
            );
        }
        other => panic!(
            "a real s2n-quic server did not answer the probe: {other:?}. \
             The hand-rolled Initial is being dropped before the version check, \
             so NegotiationProbe is dead code in production."
        ),
    }
}

/// A UDP port with nothing listening must not report as reachable. Paired with the
/// test above, this is what shows the probe discriminates rather than always
/// answering one way.
#[tokio::test(flavor = "multi_thread")]
async fn an_unused_udp_port_does_not_answer_the_probe() {
    let unused = EmbeddedServer::free_port_udp();
    let dest = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), unused);

    assert!(!NegotiationProbe::probe(dest).await.answered());
}
