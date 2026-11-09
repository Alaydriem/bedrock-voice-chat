use std::sync::Arc;

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    CertificateError, DigitallySignedStruct, Error as RustlsError, RootCertStore, SignatureScheme,
};
use x509_parser::extensions::GeneralName;

use super::PlayRoots;
use crate::errors::PlayZoneError;

/// Verifies a server certificate under the play-zone rules.
///
/// A play name is verified against the play roots only, and its leaf may name nothing
/// outside the zone. Any other name goes to `fallback`, and is then refused if its chain
/// touches a play root, so a play root added to the fallback by mistake still cannot vouch
/// for a name outside the zone.
#[derive(Debug)]
pub struct PlayZoneVerifier {
    // `None` when there are no play roots: every play name is then refused.
    play_verifier: Option<Arc<WebPkiServerVerifier>>,
    provider: Arc<CryptoProvider>,
    fallback: Arc<dyn ServerCertVerifier>,
    // Each play root with its raw subject, read once in `new`. The subject narrows the roots
    // a chain certificate can have been signed by, so a root is parsed again only on a match.
    play_roots: Vec<(Vec<u8>, CertificateDer<'static>)>,
}

impl PlayZoneVerifier {
    pub fn new(
        play_roots: Vec<CertificateDer<'static>>,
        fallback: Arc<dyn ServerCertVerifier>,
    ) -> Result<Self, PlayZoneError> {
        let mut store = RootCertStore::empty();
        let mut subjects = Vec::with_capacity(play_roots.len());
        for root in &play_roots {
            let (_, parsed) = x509_parser::parse_x509_certificate(root.as_ref())
                .map_err(|source| PlayZoneError::PlayRoot(source.to_string()))?;
            subjects.push((parsed.subject().as_raw().to_vec(), root.clone()));
            store
                .add(root.clone())
                .map_err(|source| PlayZoneError::PlayRoot(source.to_string()))?;
        }

        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let play_verifier = if play_roots.is_empty() {
            None
        } else {
            Some(
                WebPkiServerVerifier::builder_with_provider(Arc::new(store), provider.clone())
                    .build()?,
            )
        };

        Ok(Self {
            play_verifier,
            provider,
            fallback,
            play_roots: subjects,
        })
    }

    pub fn new_shared(
        play_roots: Vec<CertificateDer<'static>>,
        fallback: Arc<dyn ServerCertVerifier>,
    ) -> Result<Arc<Self>, PlayZoneError> {
        Ok(Arc::new(Self::new(play_roots, fallback)?))
    }

    fn refuse(reason: &str) -> RustlsError {
        curia::warn!("the play-zone verifier refused a certificate", {
            "reason": reason.to_string()
        });
        RustlsError::InvalidCertificate(CertificateError::ApplicationVerificationFailure)
    }

    // Every SAN must be a DNS name inside the zone. An IP, URI, e-mail or other SAN is
    // refused outright, so a leaf that also names something else cannot ride a play chain.
    fn require_zone_sans(end_entity: &CertificateDer<'_>) -> Result<(), RustlsError> {
        let (_, leaf) = x509_parser::parse_x509_certificate(end_entity.as_ref())
            .map_err(|_| Self::refuse("the leaf could not be parsed"))?;
        let san = leaf
            .subject_alternative_name()
            .map_err(|_| Self::refuse("the leaf's SAN extension is malformed"))?
            .ok_or_else(|| Self::refuse("the leaf has no SAN"))?;

        for name in &san.value.general_names {
            match name {
                GeneralName::DNSName(dns) if PlayRoots::is_zone_name(dns) => {}
                _ => return Err(Self::refuse("the leaf names something outside the zone")),
            }
        }

        Ok(())
    }

    // A certificate touches a play root when it is signed by one or is one. The subject
    // match narrows the candidates and the signature check is what decides.
    fn chain_touches_play_root(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
    ) -> Result<bool, RustlsError> {
        // A build with no play roots has nothing to protect, so a chain certificate that
        // x509-parser rejects must not fail a public server.
        if self.play_roots.is_empty() {
            return Ok(false);
        }

        for der in std::iter::once(end_entity).chain(intermediates) {
            let (_, cert) = x509_parser::parse_x509_certificate(der.as_ref())
                .map_err(|_| Self::refuse("a chain certificate could not be parsed"))?;

            for (subject, root) in &self.play_roots {
                if cert.issuer().as_raw() != subject.as_slice() {
                    continue;
                }

                let (_, root) = x509_parser::parse_x509_certificate(root.as_ref())
                    .map_err(|_| Self::refuse("a play root could not be parsed"))?;
                if cert.verify_signature(Some(root.public_key())).is_ok() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

impl ServerCertVerifier for PlayZoneVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        if PlayRoots::is_play_name(server_name) {
            let Some(play_verifier) = &self.play_verifier else {
                return Err(Self::refuse("no play root is installed"));
            };
            let verified = play_verifier.verify_server_cert(
                end_entity,
                intermediates,
                server_name,
                ocsp_response,
                now,
            )?;
            Self::require_zone_sans(end_entity)?;

            return Ok(verified);
        }

        let verified = self.fallback.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        )?;
        if self.chain_touches_play_root(end_entity, intermediates)? {
            return Err(Self::refuse(
                "a play root cannot vouch for a name outside the zone",
            ));
        }

        Ok(verified)
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
