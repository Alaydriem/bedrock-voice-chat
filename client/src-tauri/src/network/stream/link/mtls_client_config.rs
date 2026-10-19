use super::VoiceLinkError;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;

/// The TLS client configuration every socket to the server uses: the public roots plus the
/// server's own CA, presenting this player's certificate.
///
/// ALPN is the caller's: the voice link offers the voice protocol so the demultiplexer routes
/// it to the voice listener, and a protected group's key socket offers `http/1.1` so the same
/// port routes it to the API.
pub(crate) struct MtlsClientConfig;

impl MtlsClientConfig {
    /// The additional-root shape mirrors the HTTP client (`api/client.rs`): a hosted server
    /// presents a publicly-signed certificate, a self-hosted one presents a CA-signed
    /// certificate, and both must work without an operator choosing.
    pub(crate) fn build(
        ca_cert: &str,
        cert: &str,
        key: &str,
        alpn: &[&[u8]],
    ) -> Result<ClientConfig, VoiceLinkError> {
        let mut roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        for certificate in Self::parse_certificates(ca_cert)? {
            roots
                .add(certificate)
                .map_err(|source| VoiceLinkError::TrustRoot { source })?;
        }

        // Named explicitly rather than taken from the process default: whether one is
        // installed depends on which other component initialised rustls first, and
        // `ClientConfig::builder` panics rather than erroring when none is.
        let builder = ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|source| VoiceLinkError::TlsConfig { source })?
        .with_root_certificates(roots);

        let mut config = builder
            .with_client_auth_cert(Self::parse_certificates(cert)?, Self::parse_key(key)?)
            .map_err(|source| VoiceLinkError::TlsConfig { source })?;

        config.alpn_protocols = alpn.iter().map(|protocol| protocol.to_vec()).collect();

        // Matches the HTTP client, which sets `danger_accept_invalid_certs` under the same
        // condition. Development and the end-to-end harness both run servers whose
        // certificates no root will vouch for; a release build validates normally.
        #[cfg(debug_assertions)]
        {
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(super::PermissiveServerVerifier::new()));
        }

        Ok(config)
    }

    fn parse_certificates(pem: &str) -> Result<Vec<CertificateDer<'static>>, VoiceLinkError> {
        rustls_pemfile::certs(&mut pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| VoiceLinkError::ParseCertificates { source })
    }

    fn parse_key(pem: &str) -> Result<PrivateKeyDer<'static>, VoiceLinkError> {
        rustls_pemfile::private_key(&mut pem.as_bytes())
            .map_err(|source| VoiceLinkError::ParseKey { source })?
            .ok_or(VoiceLinkError::MissingPrivateKey)
    }
}
