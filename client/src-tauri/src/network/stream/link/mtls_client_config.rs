use super::VoiceLinkError;
use common::rustls::{BvcClientTls, VerifierMode};
use rustls::ClientConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

/// The TLS client configuration every socket to the server uses: the public roots plus the
/// server's own CA, presenting this player's certificate. A play-zone name is validated
/// against the built-in play roots instead.
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
        BvcClientTls::config(
            Some(ca_cert),
            Some((Self::parse_certificates(cert)?, Self::parse_key(key)?)),
            alpn,
            VerifierMode::for_build(),
        )
        .map_err(|source| VoiceLinkError::Tls { source })
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
