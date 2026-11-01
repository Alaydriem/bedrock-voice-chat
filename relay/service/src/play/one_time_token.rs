use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use jsonwebtoken::{Algorithm, EncodingKey, Header};

use crate::config::PlayCaConfig;

use super::error::PlayError;

// The token step-ca's JWK provisioner accepts for one signing request.
//
// ES256, five minutes, one `sans` entry. step-ca spends a token even when it refuses the
// request, so a retry must mint another; `mint` never returns the same token twice
// because `jti` is random.
pub struct OneTimeToken;

impl OneTimeToken {
    const LIFETIME_SECONDS: u64 = 300;
    // Tolerates a CA clock slightly behind the registry's.
    const NOT_BEFORE_SKEW_SECONDS: u64 = 30;
    const JTI_BYTES: usize = 32;

    pub fn mint(config: &PlayCaConfig, fqdn: &str) -> Result<String, PlayError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();

        let mut jti = [0u8; Self::JTI_BYTES];
        getrandom::fill(&mut jti).expect("the system random source is available");

        let claims = serde_json::json!({
            "iss": config.provisioner,
            "aud": Self::audience(config),
            "sub": fqdn,
            "sans": [fqdn],
            "iat": now,
            "nbf": now - Self::NOT_BEFORE_SKEW_SECONDS,
            "exp": now + Self::LIFETIME_SECONDS,
            "jti": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(jti),
        });

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(config.provisioner_kid.clone());

        let key = EncodingKey::from_ec_pem(config.provisioner_key.as_bytes())
            .map_err(|e| PlayError::Signer(format!("the provisioner key is unusable: {e}")))?;

        jsonwebtoken::encode(&header, &claims, &key)
            .map_err(|e| PlayError::Signer(format!("minting the one-time token: {e}")))
    }

    // The signing endpoint the token is for.
    pub fn audience(config: &PlayCaConfig) -> String {
        format!("{}/1.0/sign", config.url.trim_end_matches('/'))
    }
}
