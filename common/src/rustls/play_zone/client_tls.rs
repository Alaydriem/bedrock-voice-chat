use std::sync::Arc;

use rustls::ClientConfig;
use rustls::RootCertStore;
use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::ServerCertVerifier;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

#[cfg(debug_assertions)]
use super::PermissiveVerifier;
use super::{PlayRoots, PlayZoneVerifier, VerifierMode};
use rustls::crypto::CryptoProvider;
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
        mode: VerifierMode,
    ) -> Result<ClientConfig, PlayZoneError> {
        Self::config_with_play_roots(
            PlayRoots::builtin()?,
            extra_roots_pem,
            identity,
            alpn,
            mode,
        )
    }

    /// As [`Self::config`], with the play roots supplied. Production passes the built-in
    /// roots; a test passes the root of the chain it built.
    pub fn config_with_play_roots(
        play_roots: Vec<CertificateDer<'static>>,
        extra_roots_pem: Option<&str>,
        identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
        alpn: &[&[u8]],
        mode: VerifierMode,
    ) -> Result<ClientConfig, PlayZoneError> {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());

        let mut roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        if let Some(pem) = extra_roots_pem {
            for certificate in PlayRoots::parse_pem(pem)? {
                roots
                    .add(certificate)
                    .map_err(|source| PlayZoneError::ExtraRoot(source.to_string()))?;
            }
        }

        // A release build has no permissive verifier at all: asking for Permissive there
        // gets Strict.
        #[cfg(debug_assertions)]
        let verifier: Arc<dyn ServerCertVerifier> = match mode {
            VerifierMode::Permissive => Arc::new(PermissiveVerifier::new(provider.clone())),
            VerifierMode::Strict => Self::strict_verifier(roots, play_roots, provider.clone())?,
        };
        #[cfg(not(debug_assertions))]
        let verifier: Arc<dyn ServerCertVerifier> = {
            let _ = mode;
            Self::strict_verifier(roots, play_roots, provider.clone())?
        };

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
        mode: VerifierMode,
    ) -> Result<reqwest::ClientBuilder, PlayZoneError> {
        Self::http_client_builder_with_play_roots(
            PlayRoots::builtin()?,
            extra_roots_pem,
            identity,
            mode,
        )
    }

    /// As [`Self::http_client_builder`], with the play roots supplied.
    #[cfg(feature = "net")]
    pub fn http_client_builder_with_play_roots(
        play_roots: Vec<CertificateDer<'static>>,
        extra_roots_pem: Option<&str>,
        identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
        mode: VerifierMode,
    ) -> Result<reqwest::ClientBuilder, PlayZoneError> {
        let tls = Self::config_with_play_roots(
            play_roots,
            extra_roots_pem,
            identity,
            Self::HTTP_ALPN,
            mode,
        )?;

        Ok(reqwest::Client::builder().use_preconfigured_tls(tls))
    }

    fn strict_verifier(
        roots: RootCertStore,
        play_roots: Vec<CertificateDer<'static>>,
        provider: Arc<CryptoProvider>,
    ) -> Result<Arc<dyn ServerCertVerifier>, PlayZoneError> {
        let fallback = WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider)
            .build()?;

        Ok(PlayZoneVerifier::new_shared(play_roots, fallback)?)
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
