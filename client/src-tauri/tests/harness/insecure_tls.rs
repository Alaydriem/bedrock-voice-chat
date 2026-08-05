use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, Error as RustlsError, SignatureScheme};

/// Trusts any certificate, for talking to the in-process test server.
///
/// The harness boots a server with a freshly generated local CA on an ephemeral port, so
/// there is nothing for a client to have trusted in advance. This exists only so a test can
/// open a WebSocket to it; the same job `danger_accept_invalid_certs` already does for the
/// harness's reqwest calls.
///
/// It lives in the test tree and is not reachable from the crate, so nothing shipping can
/// pick it up.
#[derive(Debug)]
struct TrustAnything {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for TrustAnything {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

/// A client config that verifies nothing, for the harness's own server.
pub fn trust_anything() -> ClientConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("default protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(TrustAnything { provider }))
        .with_no_client_auth()
}
