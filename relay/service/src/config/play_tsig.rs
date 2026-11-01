use std::fmt;

use base64::Engine;
use hickory_proto::rr::Name;
use hickory_proto::rr::TSigner;
use hickory_proto::rr::rdata::tsig::TsigAlgorithm;
use serde::{Deserialize, Serialize};

use super::error::ConfigError;

// The key that signs zone transfers and NOTIFY. The algorithm is always HMAC-SHA256.
#[derive(Clone, Serialize, Deserialize)]
pub struct PlayTsigConfig {
    pub name: String,
    // Base64. The deployment passes it through an environment variable.
    pub secret: String,
}

impl PlayTsigConfig {
    // Seconds either side of now that a signature is accepted for.
    const FUDGE: u16 = 300;
    // The HMAC-SHA256 output length. A shorter key weakens the MAC to the key's length.
    const MIN_KEY_BYTES: usize = 32;

    pub fn signer(&self) -> Result<TSigner, ConfigError> {
        let key = base64::engine::general_purpose::STANDARD
            .decode(self.secret.trim())
            .map_err(|e| ConfigError::Parse(format!("play.dns.tsig.secret is not base64: {e}")))?;
        if key.len() < Self::MIN_KEY_BYTES {
            return Err(ConfigError::Parse(format!(
                "play.dns.tsig.secret must decode to at least {} bytes",
                Self::MIN_KEY_BYTES
            )));
        }
        let name = Name::from_ascii(&self.name)
            .map_err(|e| ConfigError::Parse(format!("play.dns.tsig.name is not a DNS name: {e}")))?;

        TSigner::new(key, TsigAlgorithm::HmacSha256, name, Self::FUDGE)
            .map_err(|e| ConfigError::Parse(format!("play.dns.tsig: {e}")))
    }
}

// The secret never appears in a log line through a stray `{:?}`.
impl fmt::Debug for PlayTsigConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayTsigConfig")
            .field("name", &self.name)
            .field("secret", &"<redacted>")
            .finish()
    }
}
