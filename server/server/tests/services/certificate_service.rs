use bvc_server_lib::stream::quic::CertificateCommonName;
use common::Game;
use x509_parser::extensions::GeneralName;

use crate::harness::CertificateFixture;

// Modern Xbox gamertags may use non-Latin scripts. The player's identity lives in the
// Common Name, so a gamertag that cannot be a DNS name must still get a certificate.
#[test]
fn a_non_ascii_gamertag_gets_a_certificate_whose_cn_round_trips() {
    let certs = CertificateFixture::create().expect("cert fixture");

    let (cert, _key) = certs
        .service
        .sign_player_cert("テスト ユーザー", &Game::Minecraft)
        .expect("a certificate for a non-ASCII gamertag");

    assert_eq!(
        CertificateCommonName::from_der(cert.der()),
        Some("minecraft:テスト ユーザー".to_string())
    );
}

// Existing BDS players with spaces in their gamertag already carry it as a DNS name;
// that must not change.
#[test]
fn a_gamertag_with_a_space_keeps_its_dns_name() {
    let certs = CertificateFixture::create().expect("cert fixture");

    let (cert, _key) = certs
        .service
        .sign_player_cert("TTV 3RICtion", &Game::Minecraft)
        .expect("a certificate for a gamertag with a space");

    let (_, parsed) = x509_parser::parse_x509_certificate(cert.der()).expect("parse");
    let san = parsed
        .subject_alternative_name()
        .expect("san extension parses")
        .expect("san extension present");
    let dns_names: Vec<&str> = san
        .value
        .general_names
        .iter()
        .filter_map(|name| match name {
            GeneralName::DNSName(dns) => Some(*dns),
            _ => None,
        })
        .collect();

    assert!(
        dns_names.contains(&"TTV 3RICtion"),
        "dns names were {dns_names:?}"
    );
}
