use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use common::curia;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;

use super::WebSocketListener;

/// The WebSocket voice listener's server certificate, re-readable while it serves.
///
/// A `ServerConfig` built with a single certificate keeps it for the life of the listener.
/// A renewal bounces only the HTTP listener, so a server that outlived its certificate would
/// go on presenting the old one on the voice transport. Handshakes consult this resolver
/// instead, and a renewal swaps the certificate under it.
#[derive(Debug)]
pub struct ReloadableCertResolver {
    certificate_path: String,
    key_path: String,
    current: RwLock<Arc<CertifiedKey>>,
}

impl ReloadableCertResolver {
    pub fn load(certificate_path: &str, key_path: &str) -> Result<Arc<Self>> {
        let current = Self::read(certificate_path, key_path)?;

        Ok(Arc::new(Self {
            certificate_path: certificate_path.to_string(),
            key_path: key_path.to_string(),
            current: RwLock::new(Arc::new(current)),
        }))
    }

    /// Re-reads both files. On error the current certificate stays in place and the error is
    /// logged, so a half-written renewal cannot leave the listener with nothing to present.
    pub fn reload(&self) -> Result<()> {
        match Self::read(&self.certificate_path, &self.key_path) {
            Ok(next) => {
                *self
                    .current
                    .write()
                    .map_err(|_| anyhow::anyhow!("the WebSocket certificate lock is poisoned"))? =
                    Arc::new(next);
                curia::info!("WebSocket voice certificate reloaded", { "certificate": self.certificate_path.clone() });
                Ok(())
            }
            Err(e) => {
                curia::error!("could not reload the WebSocket voice certificate; keeping the current one", { "error": format!("{e:#}"), "certificate": self.certificate_path.clone() });
                Err(e)
            }
        }
    }

    fn read(certificate_path: &str, key_path: &str) -> Result<CertifiedKey> {
        let chain = WebSocketListener::load_certificates(certificate_path)
            .with_context(|| format!("reading the certificate chain at {certificate_path}"))?;
        let key = WebSocketListener::load_key(key_path)
            .with_context(|| format!("reading the private key at {key_path}"))?;
        let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&key)
            .context("the private key is not a supported type")?;

        let certified = CertifiedKey::new(chain, signing_key);
        // A renewal replaces the certificate and the key as two files. Reading between the
        // two writes would otherwise pair a new certificate with the old key.
        certified
            .keys_match()
            .context("the certificate and the private key do not belong together")?;

        Ok(certified)
    }
}

impl ResolvesServerCert for ReloadableCertResolver {
    fn resolve(&self, _client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        self.current.read().ok().map(|current| current.clone())
    }
}
