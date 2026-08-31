use bvc_relay_service::storage::CertificateMaterial;
use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use time::{Duration, OffsetDateTime};

// A certificate chain, in memory.
//
// A CA and a leaf signed by it, not one self-signed certificate. A trust anchor has to
// be a CA, so a certificate serving as its own root is rejected during verification —
// and the rejection reads as a name or issuer problem rather than as a malformed
// fixture. This shape also matches what a certificate authority actually returns.
//
// No files: nothing under test reads a path any more, so a fixture that wrote one would
// be testing a mechanism that no longer exists.
pub struct CertificateFixture {
    pub material: CertificateMaterial,
    // What a client has to trust to reach the leaf.
    pub ca_pem: String,
}

impl CertificateFixture {
    // The leaf's validity window is the parameter under test twice over: the reload
    // test cares only about the name, and the renewal check cares only about the
    // window.
    pub fn issue(name: &str, valid_for: Duration) -> Self {
        let ca_key = KeyPair::generate().expect("a CA key pair");
        let mut ca_params =
            CertificateParams::new(vec![format!("ca.{name}")]).expect("valid CA parameters");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.not_before = OffsetDateTime::now_utc() - Duration::days(1);
        ca_params.not_after = OffsetDateTime::now_utc() + Duration::days(3650);
        let ca_cert = ca_params.self_signed(&ca_key).expect("a self-signed CA");
        let issuer = Issuer::new(ca_params, ca_key);

        let leaf_key = KeyPair::generate().expect("a leaf key pair");
        let mut leaf_params =
            CertificateParams::new(vec![name.to_string()]).expect("valid leaf parameters");
        leaf_params.not_before = OffsetDateTime::now_utc() - Duration::hours(1);
        leaf_params.not_after = OffsetDateTime::now_utc() + valid_for;
        let leaf_cert = leaf_params
            .signed_by(&leaf_key, &issuer)
            .expect("a signed leaf");

        Self {
            // Leaf first, then the issuer. That is the order TLS requires and the order
            // a certificate authority returns.
            material: CertificateMaterial::new(
                format!("{}{}", leaf_cert.pem(), ca_cert.pem()),
                leaf_key.serialize_pem(),
            ),
            ca_pem: ca_cert.pem(),
        }
    }
}
