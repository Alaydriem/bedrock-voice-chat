// A certificate and its key, as PEM.
//
// Carried as bytes rather than as paths because nothing here writes them to disk:
// `RustlsConfig::from_pem` takes the material directly, so the database is the only
// place the registry's certificate ever lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertificateMaterial {
    // Leaf first, then the issuer chain, exactly as the certificate authority returned
    // it.
    pub chain_pem: String,
    pub key_pem: String,
}

impl CertificateMaterial {
    pub fn new(chain_pem: String, key_pem: String) -> Self {
        Self {
            chain_pem,
            key_pem,
        }
    }
}
