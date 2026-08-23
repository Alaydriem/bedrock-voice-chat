use x509_parser::pem::parse_x509_pem;
use x509_parser::prelude::*;

/// Reads validity dates off a stored certificate PEM.
///
/// Shared so the revocation row's `expires_at` is derived one way wherever it is written —
/// the migration backfill and the revocation service both need it, and a second copy would
/// be free to drift.
pub struct CertificateValidity;

impl CertificateValidity {
    /// The certificate's `notAfter` as a Unix timestamp in seconds.
    ///
    /// `None` for anything that is not a parseable certificate. This runs over whatever is
    /// stored in the database, which may be corrupt, so absence has to be representable.
    pub fn not_after(pem: &str) -> Option<i64> {
        let (_, parsed) = parse_x509_pem(pem.as_bytes()).ok()?;
        let (_, certificate) = X509Certificate::from_der(&parsed.contents).ok()?;
        Some(certificate.validity().not_after.timestamp())
    }
}
