use x509_parser::prelude::*;

pub struct CertificateValidator;

impl CertificateValidator {
    /// Returns true if the given PEM-encoded certificate has expired.
    pub fn is_expired(pem_str: &str) -> Result<bool, anyhow::Error> {
        let (_, pem) = parse_x509_pem(pem_str.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse PEM: {}", e))?;
        let (_, cert) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse X.509 certificate: {}", e))?;
        Ok(!cert.validity().is_valid())
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
