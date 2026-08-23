use x509_parser::prelude::*;

pub struct CertificateValidator;

impl CertificateValidator {
    /// The instant this certificate stops being valid, as a Unix timestamp.
    ///
    /// Separated from the expiry answer because the two have different lifetimes: this is fixed
    /// for a given certificate and costs a full PEM and DER parse to obtain, while whether it has
    /// expired depends on the time of asking and costs a comparison.
    pub fn not_after(pem_str: &str) -> Result<i64, anyhow::Error> {
        let (_, pem) = parse_x509_pem(pem_str.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse PEM: {}", e))?;
        let (_, cert) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse X.509 certificate: {}", e))?;
        Ok(cert.validity().not_after.timestamp())
    }

    /// Returns true if the given PEM-encoded certificate has expired.
    pub fn is_expired(pem_str: &str) -> Result<bool, anyhow::Error> {
        Ok(Self::not_after(pem_str)? <= chrono::Utc::now().timestamp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malformed_pem() {
        let result = CertificateValidator::is_expired("not a pem");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_string() {
        let result = CertificateValidator::is_expired("");
        assert!(result.is_err());
    }
}
