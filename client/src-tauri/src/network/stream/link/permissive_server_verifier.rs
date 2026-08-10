use rustls::DigitallySignedStruct;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use std::sync::Arc;

/// Accepts any server certificate. **Debug builds only.**
///
/// Development servers and the end-to-end harness generate certificates no root will
/// vouch for, and the WebSocket transport has to reach them. The HTTP client already does
/// exactly this under the same condition (`api/client.rs` sets
/// `danger_accept_invalid_certs` under `#[cfg(debug_assertions)]`); this is the same
/// escape for a stack that has no such switch.
///
/// Signature checking is left intact — only the certificate chain is waved through — so
/// this still fails on a peer that cannot prove it holds the key it presented.
#[derive(Debug)]
pub(crate) struct PermissiveServerVerifier {
    provider: Arc<CryptoProvider>,
}

impl PermissiveServerVerifier {
    pub(crate) fn new() -> Self {
        Self {
            provider: CryptoProvider::get_default()
                .cloned()
                .unwrap_or_else(|| Arc::new(rustls::crypto::aws_lc_rs::default_provider())),
        }
    }
}

impl ServerCertVerifier for PermissiveServerVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
