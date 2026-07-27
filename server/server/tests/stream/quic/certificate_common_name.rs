use bvc_server_lib::stream::quic::CertificateCommonName;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

// Builds a self-signed leaf whose subject CN is exactly `cn`, returning its DER.
fn der_with_common_name(cn: &str) -> Vec<u8> {
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, cn.to_string());
    let mut params = CertificateParams::default();
    params.distinguished_name = dn;
    let key_pair = KeyPair::generate().expect("keypair");
    let cert = params.self_signed(&key_pair).expect("self-signed cert");
    cert.der().to_vec()
}

#[test]
fn extracts_a_player_common_name() {
    let der = der_with_common_name("minecraft:Steve");
    assert_eq!(
        CertificateCommonName::from_der(&der),
        Some("minecraft:Steve".to_string())
    );
}

// A peer CN is not a valid DNS name, so it lives only in the CN — extraction must
// return it verbatim, marker included.
#[test]
fn extracts_a_peer_common_name_verbatim() {
    let der = der_with_common_name("server::relay.bvc.io:5000");
    assert_eq!(
        CertificateCommonName::from_der(&der),
        Some("server::relay.bvc.io:5000".to_string())
    );
}

// Garbage input must be reported as absent rather than panicking, because this runs
// on data from the network.
#[test]
fn malformed_der_yields_none() {
    assert_eq!(
        CertificateCommonName::from_der(&[0xDE, 0xAD, 0xBE, 0xEF]),
        None
    );
}

#[test]
fn empty_input_yields_none() {
    assert_eq!(CertificateCommonName::from_der(&[]), None);
}
