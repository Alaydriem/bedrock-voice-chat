use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tauri_plugin_keyring::{CredentialType, CredentialValue, KeyringExt};

use common::consts::iap::IAP_KEYRING_NS;

pub struct IapKeyringService<'a> {
    app: &'a tauri::AppHandle,
}

impl<'a> IapKeyringService<'a> {
    pub fn new(app: &'a tauri::AppHandle) -> Self {
        Self { app }
    }

    pub fn store(&self, key: &str, value: &str) {
        let encoded = Self::encode_key(key);
        let _ = self.app.keyring().set(
            &encoded,
            CredentialType::Password,
            CredentialValue::Password(value.to_string()),
        );
    }

    pub fn load(&self, key: &str) -> Option<String> {
        let encoded = Self::encode_key(key);
        match self.app.keyring().get(&encoded, CredentialType::Password) {
            Ok(CredentialValue::Password(v)) => Some(v),
            _ => None,
        }
    }

    fn encode_key(key: &str) -> String {
        BASE64.encode(format!("{}/{}", IAP_KEYRING_NS, key))
    }
}
