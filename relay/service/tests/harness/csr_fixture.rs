use std::net::IpAddr;

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};

// Certificate requests for tests, built the way a server builds one.
pub struct CsrFixture;

impl CsrFixture {
    // DNS names only, and an empty subject.
    pub fn for_names(names: &[&str]) -> Vec<u8> {
        let mut params = CertificateParams::new(
            names.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
        )
        .expect("valid names");
        params.distinguished_name = DistinguishedName::new();
        Self::der(params)
    }

    pub fn with_ip(name: &str, address: IpAddr) -> Vec<u8> {
        let mut params = CertificateParams::new(vec![name.to_string()]).expect("a valid name");
        params.distinguished_name = DistinguishedName::new();
        params.subject_alt_names.push(SanType::IpAddress(address));
        Self::der(params)
    }

    pub fn with_common_name(name: &str, common_name: &str) -> Vec<u8> {
        let mut params = CertificateParams::new(vec![name.to_string()]).expect("a valid name");
        let mut subject = DistinguishedName::new();
        subject.push(DnType::CommonName, common_name);
        params.distinguished_name = subject;
        Self::der(params)
    }

    // The last byte of a signed request is the last byte of its signature.
    pub fn with_a_bad_signature(names: &[&str]) -> Vec<u8> {
        let mut der = Self::for_names(names);
        let last = der.len() - 1;
        der[last] ^= 0xff;
        der
    }

    pub fn pem(der: &[u8]) -> String {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(der);
        let mut pem = String::from("-----BEGIN CERTIFICATE REQUEST-----\n");
        for line in encoded.as_bytes().chunks(64) {
            pem.push_str(std::str::from_utf8(line).expect("ASCII"));
            pem.push('\n');
        }
        pem.push_str("-----END CERTIFICATE REQUEST-----\n");
        pem
    }

    fn der(params: CertificateParams) -> Vec<u8> {
        let key = KeyPair::generate().expect("a key generates");
        params
            .serialize_request(&key)
            .expect("the request serializes")
            .der()
            .to_vec()
    }
}
