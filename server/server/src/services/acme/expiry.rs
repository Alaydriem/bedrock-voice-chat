use std::time::Duration;

use anyhow::{Result, anyhow};

/// Reads validity out of a certificate PEM. Used both by storage (skip
/// issuance while the stored cert is fresh) and by the readiness probe.
pub struct CertificateExpiry;

impl CertificateExpiry {
    /// True when the certificate remains valid for at least `min_validity`
    /// from now. Errors on unparseable input rather than guessing.
    pub fn is_valid_for(cert_pem: &str, min_validity: Duration) -> Result<bool> {
        use x509_parser::prelude::*;

        let (_, pem) = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes())
            .map_err(|e| anyhow!("parsing certificate PEM: {e}"))?;
        let (_, cert) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| anyhow!("parsing certificate DER: {e}"))?;
        let not_after = cert.validity().not_after.timestamp();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| anyhow!("system clock before epoch: {e}"))?
            .as_secs() as i64;
        Ok(not_after - now >= min_validity.as_secs() as i64)
    }
}
