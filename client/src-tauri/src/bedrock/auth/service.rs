use std::sync::Arc;
use std::time::Duration;

use common::bedrock_protocol::auth::xbox::XboxLive;
use common::bedrock_protocol::{AuthManager, CachedToken, RealmsApi, RealmsEnvironment};
use common::consts::bedrock::{
    BEDROCK_KEYRING_KEY_REFRESH_TOKEN, BEDROCK_KEYRING_KEY_XUID, XBOX_CLIENT_ID,
};
use common::structs::bedrock::BedrockRenewal;

use crate::bedrock::{BedrockKeyringService, BedrockState};

pub struct BedrockAuthService;

impl BedrockAuthService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_auth_manager(
        &self,
        refresh_token: Option<&str>,
        xuid: &str,
        app_handle: Option<&tauri::AppHandle>,
    ) -> Arc<AuthManager> {
        let cache = moka::future::Cache::builder()
            .time_to_live(Duration::from_secs(86400))
            .max_capacity(100)
            .build();

        let manager = AuthManager::new(XBOX_CLIENT_ID, cache);

        // Live.com invalidates the old refresh token every time it issues a new one, so the
        // copy written at sign-in stops working the first time this manager refreshes. Without
        // this the keyring holds a previous generation and the next launch has to ask the
        // player for a device code.
        let manager = match app_handle {
            Some(handle) => {
                let keyring_handle = handle.clone();
                manager.with_token_rotated(move |_xuid, token| {
                    BedrockKeyringService::new(&keyring_handle)
                        .store(BEDROCK_KEYRING_KEY_REFRESH_TOKEN, token);
                })
            }
            None => manager,
        };

        let mgr = Arc::new(manager);

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

    /// Mints fresh Xbox Live tokens from the stored credential, for every surface that needs
    /// them. Never prompts: a rejected credential is reported, not repaired.
    ///
    /// Serialised on `BedrockState::renewal_lock`. Live.com rotates the refresh token on every
    /// use, so two renewals starting from the same stored token cannot both succeed — the
    /// second presents a token the first already spent and is told `invalid_grant`, which is
    /// indistinguishable from a genuinely dead credential. A player whose sign-in is fine would
    /// be asked to sign in again.
    ///
    /// The lock is never held across anything but the renewal, and `BedrockState` itself is
    /// locked only in short bursts around it, because the status poll and the state broadcast
    /// both need it while this runs.
    pub async fn renew(
        &self,
        state: &tauri::async_runtime::Mutex<BedrockState>,
        app_handle: &tauri::AppHandle,
    ) -> BedrockRenewal {
        let (lock, seen_token) = {
            let s = state.lock().await;
            (Arc::clone(&s.renewal_lock), s.refresh_token.clone())
        };

        let _guard = lock.lock().await;

        // Someone renewed while this call waited. Their result is this call's result; starting
        // a second refresh here is exactly the race the lock exists to prevent.
        let current_token = {
            let s = state.lock().await;
            if s.refresh_token != seen_token {
                return BedrockRenewal::Renewed;
            }
            s.refresh_token.clone()
        };

        let Some(refresh_token) = current_token else {
            let mut s = state.lock().await;
            s.clear_auth();
            s.reauth_required = true;
            return BedrockRenewal::ReauthRequired;
        };

        let result = RealmsApi::authenticate_refresh(
            XBOX_CLIENT_ID,
            &refresh_token,
            RealmsEnvironment::Retail,
        )
        .await;

        let (api, xbl_token, user_hash, access_token, new_refresh) = match result {
            Ok(v) => v,
            Err(e) => {
                let outcome = BedrockRenewal::from(&e);
                if matches!(outcome, BedrockRenewal::ReauthRequired) {
                    let mut s = state.lock().await;
                    s.clear_auth();
                    s.reauth_required = true;
                    BedrockKeyringService::new(app_handle).clear();
                }
                return outcome;
            }
        };

        let stored_xuid = {
            let s = state.lock().await;
            s.xuid.clone()
        };
        let xuid = match stored_xuid {
            Some(x) => x,
            None => match self.extract_xuid(&xbl_token).await {
                Ok(x) => x,
                Err(message) => return BedrockRenewal::Unavailable { message },
            },
        };

        let effective_refresh = new_refresh.or(Some(refresh_token));
        let keyring = BedrockKeyringService::new(app_handle);
        if let Some(ref rt) = effective_refresh {
            keyring.store(BEDROCK_KEYRING_KEY_REFRESH_TOKEN, rt);
        }
        keyring.store(BEDROCK_KEYRING_KEY_XUID, &xuid);

        let auth_manager =
            self.build_auth_manager(effective_refresh.as_deref(), &xuid, Some(app_handle));

        let mut s = state.lock().await;
        s.apply_auth(
            auth_manager,
            api,
            xbl_token,
            user_hash,
            access_token,
            effective_refresh,
            xuid,
        );

        BedrockRenewal::Renewed
    }

    /// Ends the Xbox Live session on this device: the tokens this process holds, and the
    /// credential that would otherwise restore them at next launch.
    ///
    /// Behind a service because two callers need it — the sign-out button, and a BVC logout,
    /// which has no user standing by to press anything.
    ///
    /// Local only. The refresh token is deleted here, not revoked at Microsoft.
    pub fn sign_out(&self, state: &mut BedrockState, app_handle: &tauri::AppHandle) {
        state.clear_auth();
        BedrockKeyringService::new(app_handle).clear();
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
