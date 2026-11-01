use rcgen::{CertificateParams, KeyPair};

/// The API client's own certificate and the server CA it is configured with. A play-named
/// server is checked against the play roots, so neither takes part in these requests.
pub struct ClientMaterial {
    pub ca_pem: String,
    pub identity_pem: String,
}

impl ClientMaterial {
    pub fn new() -> Self {
        let ca = CertificateParams::new(Vec::<String>::new())
            .unwrap()
            .self_signed(&KeyPair::generate().unwrap())
            .unwrap();
        let key = KeyPair::generate().unwrap();
        let identity = CertificateParams::new(vec!["player".to_string()])
            .unwrap()
            .self_signed(&key)
            .unwrap();

        Self {
            ca_pem: ca.pem(),
            identity_pem: format!("{}\n{}", identity.pem(), key.serialize_pem()),
        }
    }
}
