use std::sync::Arc;

use base64::Engine;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, aws_lc_rs, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use x509_parser::prelude::*;

// Trusts a server only when its leaf certificate's public key matches a pinned
// SPKI hash. Trust is the pin itself (the handshake signature is still verified
// against that key, proving possession), so no public-CA chain is required —
// works with a self-signed or CA-issued cert alike. Certificate validity is
// still enforced.
#[derive(Debug)]
pub(crate) struct SpkiPinningVerifier {
    provider: Arc<CryptoProvider>,
    pins: Vec<[u8; 32]>,
}

impl SpkiPinningVerifier {
    // SHA-256(SubjectPublicKeyInfo DER), base64 — the pinned public key(s) for
    // the Flagsmith host. Multiple entries allow rotating the key with a backup
    // pin without shipping a client update.
    const SPKI_PINS: &[&str] = &["Ae0qBh6ONPl2sGUMgJiRJE9mrho9ehVfPBPv3kI5eEo="];

    fn new() -> Self {
        Self {
            provider: Arc::new(aws_lc_rs::default_provider()),
            pins: Self::decode_pins(),
        }
    }

    fn decode_pins() -> Vec<[u8; 32]> {
        Self::SPKI_PINS
            .iter()
            .filter_map(|pin| {
                base64::engine::general_purpose::STANDARD
                    .decode(pin)
                    .ok()
                    .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            })
            .collect()
    }

    // A reqwest client that only completes TLS to the Flagsmith host if the
    // server presents a certificate whose public key matches a pinned SPKI hash.
    pub(crate) fn pinned_client() -> reqwest::Client {
        let verifier = Arc::new(Self::new());
        let provider = verifier.provider.clone();

        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("rustls supports the default protocol versions")
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();

        reqwest::Client::builder()
            .use_preconfigured_tls(config)
            .build()
            .expect("failed to build pinned reqwest client")
    }
}

impl ServerCertVerifier for SpkiPinningVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let (_, cert) = X509Certificate::from_der(end_entity.as_ref())
            .map_err(|_| rustls::Error::General("invalid server certificate".to_string()))?;

        if !cert.validity().is_valid() {
            return Err(rustls::Error::General(
                "server certificate is expired or not yet valid".to_string(),
            ));
        }

        let digest = Sha256::digest(cert.public_key().raw);
        if self.pins.iter().any(|pin| pin.as_slice() == digest.as_slice()) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "server public-key pin mismatch".to_string(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
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
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
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
