use std::sync::Arc;
use std::time::Duration;

use common::bedrock_protocol::auth::xbox::XboxLive;
use common::bedrock_protocol::{AuthManager, CachedToken};
use common::consts::bedrock::XBOX_CLIENT_ID;

pub struct BedrockAuthService;

impl BedrockAuthService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_auth_manager(&self, refresh_token: Option<&str>, xuid: &str) -> Arc<AuthManager> {
        let cache = moka::future::Cache::builder()
            .time_to_live(Duration::from_secs(86400))
            .max_capacity(100)
            .build();

        let mgr = Arc::new(AuthManager::new(XBOX_CLIENT_ID, cache));

        if let Some(rt) = refresh_token {
            let cache = mgr.cache().clone();
            let rt = rt.to_string();
            let xuid = xuid.to_string();
            tauri::async_runtime::spawn(async move {
                cache.insert(xuid, CachedToken { refresh_token: rt }).await;
            });
        }

        mgr
    }

    pub async fn extract_xuid(&self, xbl_token: &str) -> Result<String, String> {
        let xsts = XboxLive::authenticate_xsts(xbl_token, "http://xboxlive.com")
            .await
            .map_err(|e| format!("XSTS authentication failed: {}", e))?;
        xsts.display_claims
            .xui
            .first()
            .and_then(|c| c.xid.clone())
            .ok_or_else(|| "XUID not present in XSTS response".to_string())
    }
}

impl Default for BedrockAuthService {
    fn default() -> Self {
        Self::new()
    }
}
