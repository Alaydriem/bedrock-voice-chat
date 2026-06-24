use std::sync::Arc;

use common::errors::DiscordLinkError;
use common::structs::DiscordLinkStatus;
use log::{info, warn};
use tauri_plugin_store::StoreExt;

use crate::discord::trait_state::CACHE_TTL_SECS;
use crate::discord::{DiscordOAuth, DiscordRoleClient};
use crate::feature_flags::FeatureFlagService;

const KEY_ROLES: &str = "discord_roles";
const KEY_LAST_SYNC: &str = "discord_last_sync";
const KEY_LINKED: &str = "discord_linked";

pub struct DiscordLinkService {
    client_id: String,
    guild_id: String,
    redirect_uri: String,
    http: reqwest::Client,
    flags: Arc<FeatureFlagService>,
    app: tauri::AppHandle,
}

impl DiscordLinkService {
    pub fn new(
        client_id: String,
        guild_id: String,
        redirect_uri: String,
        http: reqwest::Client,
        flags: Arc<FeatureFlagService>,
        app: tauri::AppHandle,
    ) -> Self {
        Self { client_id, guild_id, redirect_uri, http, flags, app }
    }

    pub fn new_shared(
        client_id: String,
        guild_id: String,
        redirect_uri: String,
        http: reqwest::Client,
        flags: Arc<FeatureFlagService>,
        app: tauri::AppHandle,
    ) -> Arc<Self> {
        Arc::new(Self::new(client_id, guild_id, redirect_uri, http, flags, app))
    }

    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.guild_id.is_empty() && !self.redirect_uri.is_empty()
    }

    pub fn build_status(
        roles: &[String],
        last_sync: Option<i64>,
        now_secs: i64,
        configured: bool,
    ) -> DiscordLinkStatus {
        let expired = match last_sync {
            Some(t) => now_secs.saturating_sub(t) > CACHE_TTL_SECS,
            None => true,
        };
        DiscordLinkStatus {
            configured,
            linked: last_sync.is_some(),
            role_count: roles.len() as u32,
            last_synced: last_sync,
            expired,
        }
    }

    fn now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    // (roles, last_sync) read from the persisted store.
    fn read_persisted(&self) -> (Vec<String>, Option<i64>) {
        let Ok(store) = self.app.store("store.json") else {
            return (Vec::new(), None);
        };
        let roles = store
            .get(KEY_ROLES)
            .and_then(|v| v.as_array().cloned())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let last_sync = store.get(KEY_LAST_SYNC).and_then(|v| v.as_i64());
        (roles, last_sync)
    }

    fn write_persisted(
        &self,
        roles: &[String],
        last_sync: Option<i64>,
    ) -> Result<(), DiscordLinkError> {
        let store = self
            .app
            .store("store.json")
            .map_err(|e| DiscordLinkError::Http(e.to_string()))?;
        store.set(KEY_ROLES, serde_json::json!(roles));
        match last_sync {
            Some(t) => store.set(KEY_LAST_SYNC, serde_json::json!(t)),
            None => {
                let _ = store.delete(KEY_LAST_SYNC);
            }
        }
        store.set(KEY_LINKED, serde_json::json!(last_sync.is_some()));
        store.save().map_err(|e| DiscordLinkError::Http(e.to_string()))
    }

    pub fn load_persisted(&self) {
        let (roles, last_sync) = self.read_persisted();
        info!("Discord: seeding {} cached role(s)", roles.len());
        self.flags.seed_discord_roles(roles, last_sync);
    }

    pub async fn status(&self) -> DiscordLinkStatus {
        let (roles, last_sync) = self.read_persisted();
        Self::build_status(&roles, last_sync, Self::now_secs(), self.is_configured())
    }

    #[cfg(desktop)]
    pub async fn link(&self) -> Result<DiscordLinkStatus, DiscordLinkError> {
        if !self.is_configured() {
            return Err(DiscordLinkError::NotConfigured);
        }
        let state = uuid::Uuid::new_v4().as_simple().to_string();
        let authorize_url =
            DiscordOAuth::authorize_url(&self.client_id, &self.redirect_uri, &state);
        let fragment =
            DiscordOAuth::open_window(self.app.clone(), authorize_url, self.redirect_uri.clone())
                .await?;
        let (token, returned_state) = DiscordOAuth::parse_fragment(&fragment)?;
        if returned_state != state {
            return Err(DiscordLinkError::StateMismatch);
        }
        let roles = DiscordRoleClient::fetch_role_ids(&self.http, &token, &self.guild_id).await?;
        let now = Self::now_secs();
        self.write_persisted(&roles, Some(now))?;
        if let Err(e) = self.flags.update_discord_roles(roles.clone(), Some(now)).await {
            warn!("Discord: flag refresh after link failed: {e}");
        }
        Ok(Self::build_status(&roles, Some(now), now, true))
    }

    #[cfg(desktop)]
    pub async fn resync(&self) -> Result<DiscordLinkStatus, DiscordLinkError> {
        self.link().await
    }

    pub async fn unlink(&self) -> Result<DiscordLinkStatus, DiscordLinkError> {
        self.write_persisted(&[], None)?;
        if let Err(e) = self.flags.update_discord_roles(Vec::new(), None).await {
            warn!("Discord: flag refresh after unlink failed: {e}");
        }
        Ok(Self::build_status(&[], None, Self::now_secs(), self.is_configured()))
    }
}
