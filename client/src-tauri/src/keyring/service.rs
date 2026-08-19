use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use common::Game;
use common::response::LoginResponse;
use common::structs::config::Keypair;
use common::structs::permission::ServerPermissions;
use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use tauri_plugin_keyring::{CredentialType, CredentialValue, KeyringExt};

use super::KeyringFault;

pub(super) const KEY_GAMERPIC: &str = "gamerpic";
pub(super) const KEY_GAMERTAG: &str = "gamertag";
pub(super) const KEY_KEYPAIR: &str = "keypair";
pub(super) const KEY_SIGNATURE: &str = "signature";
pub(super) const KEY_CERTIFICATE: &str = "certificate";
pub(super) const KEY_CERTIFICATE_KEY: &str = "certificate_key";
pub(super) const KEY_CERTIFICATE_CA: &str = "certificate_ca";
pub(super) const KEY_QUIC_CONNECT_STRING: &str = "quic_connect_string";
pub(super) const KEY_SERVER_PERMISSIONS: &str = "server_permissions";
pub(super) const KEY_MINECRAFT_USERNAME: &str = "minecraft_username";
pub(super) const KEY_GAME: &str = "game";

const ALL_CREDENTIAL_KEYS: &[&str] = &[
    KEY_GAMERPIC,
    KEY_GAMERTAG,
    KEY_KEYPAIR,
    KEY_SIGNATURE,
    KEY_CERTIFICATE,
    KEY_CERTIFICATE_KEY,
    KEY_CERTIFICATE_CA,
    KEY_QUIC_CONNECT_STRING,
    KEY_SERVER_PERMISSIONS,
    KEY_MINECRAFT_USERNAME,
    KEY_GAME,
];

pub struct KeyringService {
    app_handle: AppHandle,
    cache: HashMap<String, LoginResponse>,
    // When each server's certificate stops being valid. The parse that produces it is the
    // expensive half and its answer never changes; whether the certificate has expired changes
    // with the clock, so only that is recomputed.
    cert_expiry: HashMap<String, i64>,
}

impl KeyringService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            cache: HashMap::new(),
            cert_expiry: HashMap::new(),
        }
    }

    pub fn initialize(&self) -> Result<(), anyhow::Error> {
        let identifier = self.app_handle.config().identifier.clone();
        let service_name = format!("{}-servers", identifier);
        self.app_handle
            .keyring()
            .initialize_service(service_name)
            .map_err(|e| anyhow::anyhow!("Failed to initialize keyring service: {}", e))
    }

    /// Write one server's credentials, or leave the keystore as it was.
    ///
    /// A failure rolls back the keys already written and leaves the in-memory cache untouched.
    /// Both matter: a half-written identity reads back partly present, and a cache populated
    /// after a failed write would make the running session look saved when nothing was.
    ///
    /// The error is prefixed with the fault code the error route renders, so the webview
    /// classifies on a code this crate owns rather than on a platform message.
    pub fn store_credentials(
        &mut self,
        server: &str,
        response: &LoginResponse,
    ) -> Result<(), anyhow::Error> {
        let entries = super::CredentialWriteSet::build(response)?;

        for (index, (key, value)) in entries.iter().enumerate() {
            if let Err(e) = self.set_keyring_password(server, key, value) {
                let message = e.to_string();

                for (written, _) in entries.iter().take(index) {
                    let _ = self.delete_keyring_password(server, written);
                }

                return Err(anyhow::anyhow!(
                    "{}: {}",
                    KeyringFault::label(&message),
                    message
                ));
            }
        }

        self.cache.insert(server.to_string(), response.clone());
        self.cert_expiry.remove(server);
        Ok(())
    }

    /// The server a launch will ask about, or `None` when there is nothing saved.
    ///
    /// `current_server` first, because a client with several saved servers reads credentials for
    /// whichever one it opens; the single entry is the fallback for an install that has never
    /// recorded a current one.
    fn launch_server(app: &AppHandle) -> Option<String> {
        use tauri_plugin_store::StoreExt;

        let store = app.store("store.json").ok()?;

        if let Some(current) = store
            .get("current_server")
            .and_then(|v| v.as_str().map(str::to_string))
        {
            return Some(current);
        }

        store
            .get("server_list")
            .and_then(|v| v.as_array().cloned())
            .and_then(|list| list.first().cloned())
            .and_then(|entry| entry.get("server")?.as_str().map(str::to_string))
    }

    /// Read one server's credentials so the platform keystore initialises off the launch path.
    ///
    /// On Android the first touch of the keystore costs hundreds of milliseconds whatever is
    /// read, and it lands in the launch route with the webview already waiting on it. Started
    /// here it overlaps with the bundle parse instead, and the read fills the cache the launch
    /// route then hits.
    ///
    /// Absent credentials are the expected first-run state, so a failure is logged at debug and
    /// otherwise ignored.
    ///
    /// Goes through the managed service rather than a private instance, so the launch route gets
    /// a cache hit instead of eleven warm keystore lookups.
    pub async fn warm(app: AppHandle) {
        let Some(server) = Self::launch_server(&app) else {
            return;
        };

        let state = app.state::<tauri::async_runtime::Mutex<Self>>();
        if let Err(e) = state.lock().await.get_credentials(&server) {
            log::debug!("Keyring warm-up read failed for {}: {}", server, e);
        }
    }

    pub fn get_credentials(&mut self, server: &str) -> Result<LoginResponse, anyhow::Error> {
        if let Some(cached) = self.cache.get(server) {
            return Ok(cached.clone());
        }

        let response = self.load_credentials_from_keyring(server)?;
        self.cache.insert(server.to_string(), response.clone());
        Ok(response)
    }

    pub fn get_credential(&mut self, server: &str, key: &str) -> Result<String, anyhow::Error> {
        // For standard LoginResponse fields, try cache first
        if let Some(cached) = self.cache.get(server) {
            if let Some(value) = Self::extract_field(cached, key) {
                return Ok(value);
            }
        }

        self.get_keyring_password(server, key)
    }

    pub fn set_credential(
        &mut self,
        server: &str,
        key: &str,
        value: &str,
    ) -> Result<(), anyhow::Error> {
        self.set_keyring_password(server, key, value)?;
        self.patch_cache(server, key, value);
        Ok(())
    }

    pub fn is_certificate_expired(&mut self, server: &str) -> Result<bool, anyhow::Error> {
        let not_after = match self.cert_expiry.get(server) {
            Some(not_after) => *not_after,
            None => {
                let cert_pem = self.get_credential(server, KEY_CERTIFICATE)?;
                let not_after = super::CertificateValidator::not_after(&cert_pem)?;
                self.cert_expiry.insert(server.to_string(), not_after);
                not_after
            }
        };

        Ok(not_after <= chrono::Utc::now().timestamp())
    }

    pub fn delete_credentials(&mut self, server: &str) -> Result<(), anyhow::Error> {
        for key in ALL_CREDENTIAL_KEYS {
            let _ = self.delete_keyring_password(server, key);
        }

        self.cache.remove(server);
        self.cert_expiry.remove(server);
        Ok(())
    }

    fn make_key(server: &str, key: &str) -> String {
        BASE64.encode(format!("{}/{}", server, key))
    }

    fn set_keyring_password(
        &self,
        server: &str,
        key: &str,
        value: &str,
    ) -> Result<(), anyhow::Error> {
        let encoded_key = Self::make_key(server, key);
        self.app_handle
            .keyring()
            .set(
                &encoded_key,
                CredentialType::Password,
                CredentialValue::Password(value.to_string()),
            )
            .map_err(|e| anyhow::anyhow!("Failed to set keyring password for {}: {}", key, e))
    }

    fn get_keyring_password(&self, server: &str, key: &str) -> Result<String, anyhow::Error> {
        let encoded_key = Self::make_key(server, key);
        match self
            .app_handle
            .keyring()
            .get(&encoded_key, CredentialType::Password)
        {
            Ok(CredentialValue::Password(password)) => Ok(password),
            Ok(_) => Err(anyhow::anyhow!("Unexpected credential type for {}", key)),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to get keyring password for {}: {}",
                key,
                e
            )),
        }
    }

    fn delete_keyring_password(&self, server: &str, key: &str) -> Result<(), anyhow::Error> {
        let encoded_key = Self::make_key(server, key);
        self.app_handle
            .keyring()
            .delete(&encoded_key, CredentialType::Password)
            .map_err(|e| anyhow::anyhow!("Failed to delete keyring password for {}: {}", key, e))
    }

    fn load_credentials_from_keyring(&self, server: &str) -> Result<LoginResponse, anyhow::Error> {
        let gamerpic = self.get_keyring_password(server, KEY_GAMERPIC)?;
        let gamertag = self.get_keyring_password(server, KEY_GAMERTAG)?;
        let keypair: Keypair =
            serde_json::from_str(&self.get_keyring_password(server, KEY_KEYPAIR)?)?;
        let signature: Keypair =
            serde_json::from_str(&self.get_keyring_password(server, KEY_SIGNATURE)?)?;
        let certificate = self.get_keyring_password(server, KEY_CERTIFICATE)?;
        let certificate_key = self.get_keyring_password(server, KEY_CERTIFICATE_KEY)?;
        let certificate_ca = self.get_keyring_password(server, KEY_CERTIFICATE_CA)?;
        let quic_connect_string = self.get_keyring_password(server, KEY_QUIC_CONNECT_STRING)?;

        let server_permissions: Option<ServerPermissions> = self
            .get_keyring_password(server, KEY_SERVER_PERMISSIONS)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());

        let minecraft_username = self
            .get_keyring_password(server, KEY_MINECRAFT_USERNAME)
            .ok();

        // Absent for any identity stored before the game was part of a login response.
        // None is honest there — the keyring genuinely does not know — and the next
        // successful login writes it.
        let game = self
            .get_keyring_password(server, KEY_GAME)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());

        Ok(LoginResponse {
            gamerpic,
            gamertag,
            keypair,
            signature,
            certificate,
            certificate_key,
            certificate_ca,
            quic_connect_string,
            server_permissions,
            minecraft_username,
            game,
        })
    }

    fn extract_field(response: &LoginResponse, key: &str) -> Option<String> {
        match key {
            KEY_GAMERPIC => Some(response.gamerpic.clone()),
            KEY_GAMERTAG => Some(response.gamertag.clone()),
            KEY_KEYPAIR => serde_json::to_string(&response.keypair).ok(),
            KEY_SIGNATURE => serde_json::to_string(&response.signature).ok(),
            KEY_CERTIFICATE => Some(response.certificate.clone()),
            KEY_CERTIFICATE_KEY => Some(response.certificate_key.clone()),
            KEY_CERTIFICATE_CA => Some(response.certificate_ca.clone()),
            KEY_QUIC_CONNECT_STRING => Some(response.quic_connect_string.clone()),
            KEY_SERVER_PERMISSIONS => response
                .server_permissions
                .as_ref()
                .and_then(|p| serde_json::to_string(p).ok()),
            KEY_MINECRAFT_USERNAME => response.minecraft_username.clone(),
            KEY_GAME => response
                .game
                .as_ref()
                .and_then(|g| serde_json::to_string(g).ok()),
            _ => None,
        }
    }

    fn patch_cache(&mut self, server: &str, key: &str, value: &str) {
        if let Some(cached) = self.cache.get_mut(server) {
            match key {
                KEY_GAMERPIC => cached.gamerpic = value.to_string(),
                KEY_GAMERTAG => cached.gamertag = value.to_string(),
                KEY_KEYPAIR => {
                    if let Ok(kp) = serde_json::from_str::<Keypair>(value) {
                        cached.keypair = kp;
                    }
                }
                KEY_SIGNATURE => {
                    if let Ok(kp) = serde_json::from_str::<Keypair>(value) {
                        cached.signature = kp;
                    }
                }
                KEY_CERTIFICATE => cached.certificate = value.to_string(),
                KEY_CERTIFICATE_KEY => cached.certificate_key = value.to_string(),
                KEY_CERTIFICATE_CA => cached.certificate_ca = value.to_string(),
                KEY_QUIC_CONNECT_STRING => cached.quic_connect_string = value.to_string(),
                KEY_SERVER_PERMISSIONS => {
                    cached.server_permissions =
                        serde_json::from_str::<ServerPermissions>(value).ok();
                }
                KEY_MINECRAFT_USERNAME => {
                    cached.minecraft_username = Some(value.to_string());
                }
                KEY_GAME => {
                    cached.game = serde_json::from_str::<Game>(value).ok();
                }
                _ => {}
            }
        }
    }
}
