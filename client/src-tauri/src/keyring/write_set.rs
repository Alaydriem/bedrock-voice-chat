use common::response::LoginResponse;

use super::service::{
    KEY_CERTIFICATE, KEY_CERTIFICATE_CA, KEY_CERTIFICATE_KEY, KEY_GAME, KEY_GAMERPIC, KEY_GAMERTAG,
    KEY_KEYPAIR, KEY_MINECRAFT_USERNAME, KEY_QUIC_CONNECT_STRING, KEY_SERVER_PERMISSIONS,
    KEY_SIGNATURE,
};

/// Every credential a login writes, resolved to plain strings before the first keystore call.
///
/// Serialization ahead of the write loop is what makes the write all-or-nothing: a `serde_json`
/// failure on the fifth field used to abort after four keys were already persisted, leaving an
/// identity that reads back partly present.
pub struct CredentialWriteSet;

impl CredentialWriteSet {
    pub fn build(response: &LoginResponse) -> Result<Vec<(&'static str, String)>, anyhow::Error> {
        let mut entries = vec![
            (KEY_GAMERPIC, response.gamerpic.clone()),
            (KEY_GAMERTAG, response.gamertag.clone()),
            (KEY_KEYPAIR, serde_json::to_string(&response.keypair)?),
            (KEY_SIGNATURE, serde_json::to_string(&response.signature)?),
            (KEY_CERTIFICATE, response.certificate.clone()),
            (KEY_CERTIFICATE_KEY, response.certificate_key.clone()),
            (KEY_CERTIFICATE_CA, response.certificate_ca.clone()),
            (
                KEY_QUIC_CONNECT_STRING,
                response.quic_connect_string.clone(),
            ),
        ];

        if let Some(ref perms) = response.server_permissions {
            entries.push((KEY_SERVER_PERMISSIONS, serde_json::to_string(perms)?));
        }

        if let Some(ref mc_username) = response.minecraft_username {
            entries.push((KEY_MINECRAFT_USERNAME, mc_username.clone()));
        }

        // The game is the one part of an identity a code login cannot reconstruct from anything
        // else, since the client sent only a code. A re-auth that read it back as None would
        // silently downgrade to a guess.
        if let Some(ref game) = response.game {
            entries.push((KEY_GAME, serde_json::to_string(game)?));
        }

        Ok(entries)
    }
}
