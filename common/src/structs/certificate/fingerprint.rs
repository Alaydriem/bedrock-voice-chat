use sha2::{Digest, Sha256};
use x509_parser::pem::parse_x509_pem;

/// SHA-256 of a certificate's leaf DER, as lowercase hex.
///
/// Every caller that identifies a certificate uses this, so a fingerprint written by the ban
/// path and one computed at a handshake are the same string by construction.
pub struct CertificateFingerprint;

impl CertificateFingerprint {
    pub fn from_der(der: &[u8]) -> String {
        use std::fmt::Write;

        let digest = Sha256::digest(der);
        let mut out = String::with_capacity(digest.len() * 2);
        for byte in digest {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    /// `None` when the input is not a parseable PEM block.
    ///
    /// Never falls back to hashing the raw string: a stored certificate that fails to parse
    /// must produce no fingerprint at all rather than one that could collide.
    pub fn from_pem(pem: &str) -> Option<String> {
        let (_, parsed) = parse_x509_pem(pem.as_bytes()).ok()?;
        Some(Self::from_der(&parsed.contents))
    }
}
