use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use common::response::LoginResponse;
use common::structs::config::Keypair;
use common::structs::permission::ServerPermissions;
use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use tauri_plugin_keyring::{CredentialType, CredentialValue, KeyringExt};

const KEY_GAMERPIC: &str = "gamerpic";
const KEY_GAMERTAG: &str = "gamertag";
const KEY_KEYPAIR: &str = "keypair";
const KEY_SIGNATURE: &str = "signature";
const KEY_CERTIFICATE: &str = "certificate";
const KEY_CERTIFICATE_KEY: &str = "certificate_key";
const KEY_CERTIFICATE_CA: &str = "certificate_ca";
const KEY_QUIC_CONNECT_STRING: &str = "quic_connect_string";
const KEY_SERVER_PERMISSIONS: &str = "server_permissions";
const KEY_MINECRAFT_USERNAME: &str = "minecraft_username";

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
];

pub struct KeyringService {
    app_handle: AppHandle,
    cache: HashMap<String, LoginResponse>,
}

impl KeyringService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            cache: HashMap::new(),
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

    pub fn store_credentials(
        &mut self,
        server: &str,
        response: &LoginResponse,
    ) -> Result<(), anyhow::Error> {
        self.set_keyring_password(server, KEY_GAMERPIC, &response.gamerpic)?;
        self.set_keyring_password(server, KEY_GAMERTAG, &response.gamertag)?;
        self.set_keyring_password(
            server,
            KEY_KEYPAIR,
            &serde_json::to_string(&response.keypair)?,
        )?;
        self.set_keyring_password(
            server,
            KEY_SIGNATURE,
            &serde_json::to_string(&response.signature)?,
        )?;
        self.set_keyring_password(server, KEY_CERTIFICATE, &response.certificate)?;
        self.set_keyring_password(server, KEY_CERTIFICATE_KEY, &response.certificate_key)?;
        self.set_keyring_password(server, KEY_CERTIFICATE_CA, &response.certificate_ca)?;
        self.set_keyring_password(server, KEY_QUIC_CONNECT_STRING, &response.quic_connect_string)?;

        if let Some(ref perms) = response.server_permissions {
            self.set_keyring_password(
                server,
                KEY_SERVER_PERMISSIONS,
                &serde_json::to_string(perms)?,
            )?;
        }

        if let Some(ref mc_username) = response.minecraft_username {
            self.set_keyring_password(server, KEY_MINECRAFT_USERNAME, mc_username)?;
        }

        self.cache.insert(server.to_string(), response.clone());
        Ok(())
    }

    pub fn get_credentials(
        &mut self,
        server: &str,
    ) -> Result<LoginResponse, anyhow::Error> {
        if let Some(cached) = self.cache.get(server) {
            return Ok(cached.clone());
        }

        let response = self.load_credentials_from_keyring(server)?;
        self.cache.insert(server.to_string(), response.clone());
        Ok(response)
    }

    pub fn get_credential(
        &mut self,
        server: &str,
        key: &str,
    ) -> Result<String, anyhow::Error> {
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

    pub fn delete_credentials(
        &mut self,
        server: &str,
    ) -> Result<(), anyhow::Error> {
        for key in ALL_CREDENTIAL_KEYS {
            let _ = self.delete_keyring_password(server, key);
        }

        self.cache.remove(server);
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

    fn get_keyring_password(
        &self,
        server: &str,
        key: &str,
    ) -> Result<String, anyhow::Error> {
        let encoded_key = Self::make_key(server, key);
        match self.app_handle.keyring().get(&encoded_key, CredentialType::Password) {
            Ok(CredentialValue::Password(password)) => Ok(password),
            Ok(_) => Err(anyhow::anyhow!("Unexpected credential type for {}", key)),
            Err(e) => Err(anyhow::anyhow!("Failed to get keyring password for {}: {}", key, e)),
        }
    }

    fn delete_keyring_password(
        &self,
        server: &str,
        key: &str,
    ) -> Result<(), anyhow::Error> {
        let encoded_key = Self::make_key(server, key);
        self.app_handle
            .keyring()
            .delete(&encoded_key, CredentialType::Password)
            .map_err(|e| anyhow::anyhow!("Failed to delete keyring password for {}: {}", key, e))
    }

    fn load_credentials_from_keyring(
        &self,
        server: &str,
    ) -> Result<LoginResponse, anyhow::Error> {
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
                    cached.server_permissions = serde_json::from_str::<ServerPermissions>(value).ok();
                }
                KEY_MINECRAFT_USERNAME => {
                    cached.minecraft_username = Some(value.to_string());
                }
                _ => {}
            }
        }
    }
}
