use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};

/// Accepts any server certificate, so the probe reaches the negotiated ALPN.
///
/// The ALPN this probe reads arrives in the same server flight as the certificate. Verifying
/// the certificate would fail that flight on a self-hosted server whose CA no public root
/// vouches for, and the probe would report no voice path on a server that has one — which is
/// the same population the WebSocket transport exists for.
///
/// Trusts everything and must never back real traffic. Only reachable through
/// `WsVoiceProbe`, which drops its socket the moment the ALPN is known and never sends
/// application data.
#[derive(Debug)]
pub struct VoiceProbeVerifier {
    provider: Arc<CryptoProvider>,
}

impl VoiceProbeVerifier {
    pub fn new() -> Self {
        Self {
            provider: Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn crypto_provider(&self) -> Arc<CryptoProvider> {
        self.provider.clone()
    }
}

impl Default for VoiceProbeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerCertVerifier for VoiceProbeVerifier {
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
        verify_tls12_signature(
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
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
