use std::sync::Arc;

use rustls::RootCertStore;
use rustls::crypto::aws_lc_rs;
use rustls::pki_types::CertificateDer;

// Trust anchored to Let's Encrypt's ISRG roots (X1 and X2), embedded as DER.
// The handshake runs standard webpki path validation against these two roots
// and nothing else, so a host is trusted only when its leaf chains up to ISRG
// X1 or X2. Because trust is anchored at the root, leaf and intermediate
// rotation (Let's Encrypt reissues both regularly) never invalidates it.
pub struct LetsEncryptRootStore;

impl LetsEncryptRootStore {
    const ISRG_ROOT_X1: &[u8] = include_bytes!("roots/isrg_root_x1.der");
    const ISRG_ROOT_X2: &[u8] = include_bytes!("roots/isrg_root_x2.der");

    pub fn root_cert_store() -> RootCertStore {
        let mut store = RootCertStore::empty();
        for der in [Self::ISRG_ROOT_X1, Self::ISRG_ROOT_X2] {
            store
                .add(CertificateDer::from(der))
                .expect("embedded ISRG root certificate is valid DER");
        }
        store
    }

    pub fn client_config() -> rustls::ClientConfig {
        let provider = Arc::new(aws_lc_rs::default_provider());
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("rustls supports the default protocol versions")
            .with_root_certificates(Self::root_cert_store())
            .with_no_client_auth()
    }
}
