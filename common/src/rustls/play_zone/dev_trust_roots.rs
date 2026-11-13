use std::path::Path;

use rustls::RootCertStore;
use rustls::pki_types::CertificateDer;

use super::PlayRoots;
use crate::errors::PlayZoneError;

/// Extra trust anchors for development servers and the end-to-end harness, which use
/// self-signed certificate authorities and are reached before a client holds the server's
/// own CA. Exists only in a debug build that enables the `dev-trust` feature.
///
/// These anchors join the ordinary root store, so they vouch for names outside the play
/// zone. A play name is validated against the play roots alone and never reaches them.
pub struct DevTrustRoots;

impl DevTrustRoots {
    /// The environment variable naming a PEM bundle of extra trust anchors, read at run time.
    pub const ENV: &'static str = "BVC_DEV_TRUST_ROOTS";

    /// The environment variable holding the PEM text of extra trust anchors, read when the
    /// binary is compiled. A mobile app cannot see its host's environment, so the bundle is
    /// baked into the binary instead.
    pub const BAKED_ENV: &'static str = "BVC_DEV_TRUST_ROOTS_PEM";

    /// The certificates from every source that is present: the bundle [`Self::ENV`] names
    /// and the PEM baked in under [`Self::BAKED_ENV`]. A source that is present but cannot be
    /// read, or holds no certificate, is an error. The run-time variable is read on every
    /// call, because the harness registers a server's CA after clients may already exist.
    pub fn from_env() -> Result<Vec<CertificateDer<'static>>, PlayZoneError> {
        let runtime = std::env::var_os(Self::ENV);

        Self::from_sources(runtime.as_deref().map(Path::new), option_env!("BVC_DEV_TRUST_ROOTS_PEM"))
    }

    /// The certificates in the bundle at `path` and in `baked_pem`, either of which may be
    /// absent. An empty `baked_pem` counts as absent.
    pub fn from_sources(
        path: Option<&Path>,
        baked_pem: Option<&str>,
    ) -> Result<Vec<CertificateDer<'static>>, PlayZoneError> {
        let mut certificates = Vec::new();
        if let Some(path) = path {
            certificates.extend(Self::from_path(path)?);
        }
        if let Some(pem) = baked_pem.filter(|pem| !pem.trim().is_empty()) {
            certificates.extend(Self::from_pem(pem, Self::BAKED_ENV)?);
        }

        Ok(certificates)
    }

    /// Every certificate in the PEM bundle at `path`.
    pub fn from_path(path: &Path) -> Result<Vec<CertificateDer<'static>>, PlayZoneError> {
        let origin = format!("{} ({})", Self::ENV, path.display());
        let pem = std::fs::read_to_string(path)
            .map_err(|source| PlayZoneError::DevRoot(format!("{origin}: {source}")))?;

        Self::from_pem(&pem, &origin)
    }

    /// Every certificate in `pem`. `origin` names the source in an error.
    pub fn from_pem(
        pem: &str,
        origin: &str,
    ) -> Result<Vec<CertificateDer<'static>>, PlayZoneError> {
        let certificates = PlayRoots::parse_pem(pem)
            .map_err(|source| PlayZoneError::DevRoot(format!("{origin}: {source}")))?;
        if certificates.is_empty() {
            return Err(PlayZoneError::DevRoot(format!(
                "{origin}: the bundle holds no certificate"
            )));
        }

        // The root store is the final judge of an anchor, so a certificate it refuses is
        // reported here, against its source, rather than as a server CA failure.
        let mut store = RootCertStore::empty();
        for certificate in &certificates {
            store
                .add(certificate.clone())
                .map_err(|source| PlayZoneError::DevRoot(format!("{origin}: {source}")))?;
        }

        Ok(certificates)
    }
}
