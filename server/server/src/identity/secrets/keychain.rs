use std::sync::OnceLock;

use anyhow::anyhow;
use keyring_core::Entry;

use super::SecretBackend;

const SERVICE_NAME: &str = "bvc-cli";
static STORE_INIT: OnceLock<()> = OnceLock::new();

pub struct KeychainBackend;

impl KeychainBackend {
    pub fn new() -> Self {
        STORE_INIT.get_or_init(|| {
            #[cfg(target_os = "windows")]
            {
                use windows_native_keyring_store::Store as WindowsStore;
                if let Ok(store) = WindowsStore::new() {
                    keyring_core::set_default_store(store);
                }
            }
            #[cfg(target_os = "macos")]
            {
                use apple_native_keyring_store::keychain::Store as MacOSStore;
                if let Ok(store) = MacOSStore::new() {
                    keyring_core::set_default_store(store);
                }
            }
        });
        KeychainBackend
    }

    fn entry(slot_key: &str, kind: &str) -> Result<Entry, anyhow::Error> {
        let username = format!("{}/{}/{}", SERVICE_NAME, slot_key, kind);
        Entry::new(SERVICE_NAME, &username).map_err(|e| anyhow!("keyring entry error: {}", e))
    }
}

impl SecretBackend for KeychainBackend {
    fn save(&self, slot_key: &str, cert: &str, key: &str, ca: &str) -> Result<(), anyhow::Error> {
        Self::entry(slot_key, "cert")?
            .set_password(cert)
            .map_err(|e| anyhow!("save cert: {}", e))?;
        Self::entry(slot_key, "key")?
            .set_password(key)
            .map_err(|e| anyhow!("save key: {}", e))?;
        Self::entry(slot_key, "ca")?
            .set_password(ca)
            .map_err(|e| anyhow!("save ca: {}", e))?;
        Ok(())
    }

    fn load(&self, slot_key: &str) -> Result<(String, String, String), anyhow::Error> {
        let cert = Self::entry(slot_key, "cert")?
            .get_password()
            .map_err(|e| anyhow!("load cert: {}", e))?;
        let key = Self::entry(slot_key, "key")?
            .get_password()
            .map_err(|e| anyhow!("load key: {}", e))?;
        let ca = Self::entry(slot_key, "ca")?
            .get_password()
            .map_err(|e| anyhow!("load ca: {}", e))?;
        Ok((cert, key, ca))
    }

    fn delete(&self, slot_key: &str) -> Result<(), anyhow::Error> {
        for kind in ["cert", "key", "ca"] {
            match Self::entry(slot_key, kind)?.delete_credential() {
                Ok(()) => {}
                Err(keyring_core::Error::NoEntry) => {}
                Err(e) => return Err(anyhow!("delete {}: {}", kind, e)),
            }
        }
        Ok(())
    }
}
