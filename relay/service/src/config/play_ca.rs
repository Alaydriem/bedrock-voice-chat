use std::fmt;

use serde::{Deserialize, Serialize};

use super::play_ca_endpoint::PlayCaEndpoint;

// How the registry reaches the private step-ca that signs play certificates.
#[derive(Clone, Serialize, Deserialize)]
pub struct PlayCaConfig {
    // The CA's API base, e.g. https://ca.play.bedrockvc.stream:9000
    pub url: String,
    // Where that name actually connects: `127.0.0.1:9000` on one host, or a compose
    // service such as `play-ca:9000`. The name in `url` is never looked up; `connect` is
    // resolved with the system resolver.
    pub connect: PlayCaEndpoint,
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
