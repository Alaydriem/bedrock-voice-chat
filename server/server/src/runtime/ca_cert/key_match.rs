use rcgen::{KeyPair, PublicKeyData};

/// Whether a certificate's public key is the one belonging to a keypair.
///
/// Compares the full SubjectPublicKeyInfo DER on both sides, so the algorithm identifier is
/// part of the comparison and two keys of different types can never appear equal.
pub struct KeyMatch;

impl KeyMatch {
    /// `false` for anything unparseable. A certificate that cannot be read is not one that
    /// corresponds, and the caller's answer to both is the same: re-sign from the key.
    pub fn matches(cert_pem: &str, keypair: &KeyPair) -> bool {
        use x509_parser::prelude::*;

        let Ok((_, parsed_pem)) = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes()) else {
            return false;
        };
        let Ok((_, cert)) = X509Certificate::from_der(&parsed_pem.contents) else {
            return false;
        };
        cert.public_key().raw == keypair.subject_public_key_info().as_slice()
    }
}
