use std::sync::Arc;

use rustls::ClientConfig;
use rustls::RootCertStore;
use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::ServerCertVerifier;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

#[cfg(all(debug_assertions, feature = "dev-trust"))]
use super::DevTrustRoots;
use super::{PlayRoots, PlayZoneVerifier};
use crate::errors::PlayZoneError;

/// The TLS client configuration every BVC client socket that can reach a BVC server uses.
pub struct BvcClientTls;

impl BvcClientTls {
    /// What reqwest offers on a configuration it builds itself. It leaves a preconfigured
    /// one as given, so a reqwest caller passes this.
    pub const HTTP_ALPN: &'static [&'static [u8]] = &[b"h2", b"http/1.1"];

    /// The public roots plus `extra_roots_pem` (the server's own CA) validate any name
    /// outside the play zone. A play name is validated against the built-in play roots.
    ///
    /// ALPN is the caller's. The provider is named rather than taken from the process
    /// default, which is ambiguous whenever more than one rustls backend is linked.
    pub fn config(
        extra_roots_pem: Option<&str>,
        identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
        alpn: &[&[u8]],
    ) -> Result<ClientConfig, PlayZoneError> {
        Self::config_with_play_roots(PlayRoots::builtin()?, extra_roots_pem, identity, alpn)
    }

    /// As [`Self::config`], with the play roots supplied. Production passes the built-in
    /// roots; a test passes the root of the chain it built.
    pub fn config_with_play_roots(
        play_roots: Vec<CertificateDer<'static>>,
        extra_roots_pem: Option<&str>,
        identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
        alpn: &[&[u8]],
    ) -> Result<ClientConfig, PlayZoneError> {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let verifier = Self::server_verifier(play_roots, Self::extra_roots(extra_roots_pem)?)?;

        let builder = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(verifier);

        let mut config = match identity {
            Some((chain, key)) => builder.with_client_auth_cert(chain, key)?,
            None => builder.with_no_client_auth(),
        };
        config.alpn_protocols = alpn.iter().map(|protocol| protocol.to_vec()).collect();

        Ok(config)
    }

    /// A reqwest client builder on [`Self::config`] with [`Self::HTTP_ALPN`]. The caller adds
    /// its own timeouts and local address.
    #[cfg(feature = "net")]
    pub fn http_client_builder(
        extra_roots_pem: Option<&str>,
        identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
    ) -> Result<reqwest::ClientBuilder, PlayZoneError> {
        Self::http_client_builder_with_play_roots(PlayRoots::builtin()?, extra_roots_pem, identity)
    }

    /// As [`Self::http_client_builder`], with the play roots supplied.
    #[cfg(feature = "net")]
    pub fn http_client_builder_with_play_roots(
        play_roots: Vec<CertificateDer<'static>>,
        extra_roots_pem: Option<&str>,
        identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
    ) -> Result<reqwest::ClientBuilder, PlayZoneError> {
        let tls =
            Self::config_with_play_roots(play_roots, extra_roots_pem, identity, Self::HTTP_ALPN)?;

        Ok(reqwest::Client::builder().use_preconfigured_tls(tls))
    }

    /// The verifier behind every client configuration. `extra_roots` join the public roots
    /// for names outside the play zone; a play name is checked against `play_roots` alone.
    pub fn server_verifier(
        play_roots: Vec<CertificateDer<'static>>,
        extra_roots: Vec<CertificateDer<'static>>,
    ) -> Result<Arc<dyn ServerCertVerifier>, PlayZoneError> {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());

        let mut roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        for certificate in extra_roots {
            roots
                .add(certificate)
                .map_err(|source| PlayZoneError::ExtraRoot(source.to_string()))?;
        }

        let fallback =
            WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider).build()?;

        Ok(PlayZoneVerifier::new_shared(play_roots, fallback)?)
    }

    #[cfg(all(debug_assertions, feature = "dev-trust"))]
    fn extra_roots(
        extra_roots_pem: Option<&str>,
    ) -> Result<Vec<CertificateDer<'static>>, PlayZoneError> {
        let mut roots = match extra_roots_pem {
            Some(pem) => PlayRoots::parse_pem(pem)?,
            None => Vec::new(),
        };
        roots.extend(DevTrustRoots::from_env()?);

        Ok(roots)
    }

    #[cfg(not(all(debug_assertions, feature = "dev-trust")))]
    fn extra_roots(
        extra_roots_pem: Option<&str>,
    ) -> Result<Vec<CertificateDer<'static>>, PlayZoneError> {
        match extra_roots_pem {
            Some(pem) => PlayRoots::parse_pem(pem),
            None => Ok(Vec::new()),
        }
    }

    /// A certificate chain and private key read from one PEM document, in any order.
    pub fn identity_from_pem(
        pem: &str,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), PlayZoneError> {
        let chain = CertificateDer::pem_slice_iter(pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| PlayZoneError::Identity(source.to_string()))?;
        let key = PrivateKeyDer::from_pem_slice(pem.as_bytes())
            .map_err(|source| PlayZoneError::Identity(source.to_string()))?;

        Ok((chain, key))
    }
}
