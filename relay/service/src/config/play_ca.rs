use std::fmt;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

// How the registry reaches the loopback step-ca that signs play certificates.
#[derive(Clone, Serialize, Deserialize)]
pub struct PlayCaConfig {
    // The CA's API base, e.g. https://ca.play.bedrockvc.stream:9000
    pub url: String,
    // Where that name actually connects: the CA's loopback listener.
    pub connect: SocketAddr,
    pub provisioner: String,
    pub provisioner_kid: String,
    // PKCS#8 PEM of the provisioner's ES256 key.
    pub provisioner_key: String,
    // Path to the play root certificate the CA's own TLS chains to.
    pub root: String,
}

// The key never appears in a log line through a stray `{:?}`.
impl fmt::Debug for PlayCaConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayCaConfig")
            .field("url", &self.url)
            .field("connect", &self.connect)
            .field("provisioner", &self.provisioner)
            .field("provisioner_kid", &self.provisioner_kid)
            .field("provisioner_key", &"<redacted>")
            .field("root", &self.root)
            .finish()
    }
}
