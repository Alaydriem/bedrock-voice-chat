use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};

/// Reads validity out of a certificate PEM. Used both by storage (skip
/// issuance while the stored cert is fresh) and by the readiness probe.
pub struct CertificateExpiry;

impl CertificateExpiry {
    /// True when the certificate remains valid for at least `min_validity`
    /// from now. Errors on unparseable input rather than guessing.
    pub fn is_valid_for(cert_pem: &str, min_validity: Duration) -> Result<bool> {
        let (_, not_after) = Self::validity(cert_pem)?;
        let now = Self::now_seconds()?;
        Ok(not_after - now >= min_validity.as_secs() as i64)
    }

    /// True when the certificate's `notAfter` has passed.
    pub fn is_expired(cert_pem: &str) -> Result<bool> {
        let (_, not_after) = Self::validity(cert_pem)?;
        Ok(not_after <= Self::now_seconds()?)
    }

    /// When to replace the certificate: two thirds of the way from `notBefore` to `notAfter`.
    ///
    /// A fraction of the lifetime rather than a fixed margin, because the lifetime is the
    /// registry's to choose. Two thirds of 15 days leaves five days to retry before expiry.
    pub fn renew_at(cert_pem: &str) -> Result<SystemTime> {
        let (not_before, not_after) = Self::validity(cert_pem)?;
        let lifetime = not_after - not_before;
        let at = not_before + lifetime * 2 / 3;

        let seconds = u64::try_from(at)
            .map_err(|_| anyhow!("certificate renewal time is before the epoch"))?;
        Ok(UNIX_EPOCH + Duration::from_secs(seconds))
    }

    // `(notBefore, notAfter)` of the first certificate in the PEM, as Unix seconds.
    fn validity(cert_pem: &str) -> Result<(i64, i64)> {
        use x509_parser::prelude::*;

        let (_, pem) = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes())
            .map_err(|e| anyhow!("parsing certificate PEM: {e}"))?;
        let (_, cert) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| anyhow!("parsing certificate DER: {e}"))?;

        Ok((
            cert.validity().not_before.timestamp(),
            cert.validity().not_after.timestamp(),
        ))
    }

    fn now_seconds() -> Result<i64> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow!("system clock before epoch: {e}"))?
            .as_secs() as i64)
    }
}
