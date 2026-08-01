use std::sync::{Arc, Mutex};

use s2n_quic::provider::tls as s2n_quic_tls_provider;
#[allow(deprecated)]
use s2n_quic::provider::tls::rustls::rustls::{
    self as rustls_crate, DigitallySignedStruct, Error as RustlsError, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};

// Records the certificate a server presents and validates nothing. Its only
// purpose is to let a probe observe a server that will reject it a moment later
// for having no client certificate.
//
// This verifier trusts everything and must never be used for real traffic. It is
// not re-exported from `crate::rustls`, so a caller reaching for `MtlsProvider`
// cannot pick it up by accident.
#[derive(Debug)]
pub struct ProbeCertVerifier {
    observed: Arc<Mutex<Option<CertificateDer<'static>>>>,
    provider: Arc<CryptoProvider>,
}

impl ProbeCertVerifier {
    pub fn new() -> (Arc<Self>, Arc<Mutex<Option<CertificateDer<'static>>>>) {
        let observed = Arc::new(Mutex::new(None));
        let verifier = Arc::new(Self {
            observed: observed.clone(),
            provider: Arc::new(rustls_crate::crypto::aws_lc_rs::default_provider()),
        });

        (verifier, observed)
    }

    pub fn crypto_provider(&self) -> Arc<CryptoProvider> {
        self.provider.clone()
    }
}

impl ServerCertVerifier for ProbeCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        if let Ok(mut slot) = self.observed.lock() {
            *slot = Some(end_entity.clone().into_owned());
        }

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

pub struct ProbeTlsProvider {
    verifier: Arc<ProbeCertVerifier>,
}

impl ProbeTlsProvider {
    pub fn new(verifier: Arc<ProbeCertVerifier>) -> Self {
        Self { verifier }
    }

    // ALPN h3 and TLS 1.3 mirror what the BVC server offers, and SNI is what lets
    // an SNI-routing proxy place the probe on the right backend. Negotiating
    // neither would fail before the server presented a certificate, and a live
    // server would be reported as silent.
    //
    // The crypto provider is supplied explicitly rather than taken from the
    // process default, which is ambiguous whenever more than one rustls backend is
    // reachable in a build and panics when it is.
    pub fn client_config(
        verifier: Arc<ProbeCertVerifier>,
    ) -> Result<rustls_crate::ClientConfig, RustlsError> {
        let provider = verifier.crypto_provider();

        let mut config = rustls_crate::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls_crate::version::TLS13])?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();

        config.alpn_protocols = vec![b"h3".to_vec()];
        config.enable_sni = true;
        Ok(config)
    }
}

impl s2n_quic_tls_provider::Provider for ProbeTlsProvider {
    type Server = s2n_quic_tls_provider::rustls::Server;
    type Client = s2n_quic_tls_provider::rustls::Client;
    type Error = RustlsError;

    // A probe never listens. Refusing here keeps a verifier that trusts everything
    // from ever backing a server.
    fn start_server(self) -> Result<Self::Server, Self::Error> {
        Err(RustlsError::General(
            "the reachability probe provider is client-only".to_string(),
        ))
    }

    fn start_client(self) -> Result<Self::Client, Self::Error> {
        Ok(Self::client_config(self.verifier)?.into())
    }
}
