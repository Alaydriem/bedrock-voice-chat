#![cfg(feature = "quic")]

use common::net::{HandshakeProbe, ProbeCertVerifier, ProbeTlsProvider};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// ALPN h3 and TLS 1.3 are what the BVC server offers. A probe that negotiated
// neither would fail before the server ever presented a certificate, and a live
// server would be reported as silent.
#[test]
fn the_probe_client_offers_what_the_server_accepts() {
    let (verifier, _observed) = ProbeCertVerifier::new();
    let config = ProbeTlsProvider::client_config(verifier).unwrap();

    assert_eq!(config.alpn_protocols, vec![b"h3".to_vec()]);
}

// SNI is the property an SNI-routing proxy needs in order to place the probe on
// the right backend, which is the only reason this layer exists alongside the
// cheaper Version Negotiation probe.
#[test]
fn the_probe_client_sends_sni_so_a_routing_proxy_can_place_it() {
    let (verifier, _observed) = ProbeCertVerifier::new();
    let config = ProbeTlsProvider::client_config(verifier).unwrap();

    assert!(config.enable_sni);
}

#[tokio::test]
async fn a_closed_port_is_silent_and_yields_no_certificate() {
    let dest = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1);
    let (outcome, certificate) = HandshakeProbe::probe(dest, "example.test").await;

    assert!(!outcome.answered());
    assert!(certificate.is_none());
}
