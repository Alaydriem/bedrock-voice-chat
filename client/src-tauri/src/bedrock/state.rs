use std::sync::Arc;

use common::bedrock_protocol::{AuthManager, RealmsApi};
use common::traits::StreamTrait;
use tokio::sync::watch;

use crate::bedrock::BedrockPlayerStateCache;
use crate::bedrock::BedrockProxyManager;
use crate::bedrock::TransferKeepAlive;
use crate::structs::app_state::AppState;

pub struct BedrockState {
    pub proxy: Option<BedrockProxyManager>,
    pub realms: Option<BedrockProxyManager>,
    pub keepalive: Option<TransferKeepAlive>,
    pub auth_manager: Option<Arc<AuthManager>>,
    pub realms_api: Option<RealmsApi>,
    pub xbl_token: Option<String>,
    pub user_hash: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub xuid: Option<String>,
    pub player_state_cache: Arc<BedrockPlayerStateCache>,
    /// Set only by a rejected root refresh. An expired XSTS or XBL token is a re-mint, and
    /// must never set this.
    pub reauth_required: bool,
    /// Held across a renewal so two callers cannot both spend the same rotating refresh
    /// token. Never held across an unrelated await.
    pub renewal_lock: Arc<tokio::sync::Mutex<()>>,
    pub login_cancel_tx: Option<watch::Sender<bool>>,
    pub proxy_target_host: Option<String>,
    pub proxy_target_port: Option<u16>,
    pub proxy_listen_port: Option<u16>,
    pub proxy_started_at: Option<u64>,
    pub active_realm_id: Option<u64>,
    pub active_realm_name: Option<String>,
    /// The world a controller sees, recorded rather than inferred so a state frame names
    /// the same entry a `targets` listing does.
    pub active_connection: Option<websocket_types::ActiveConnection>,
}

impl BedrockState {
    pub fn new() -> Self {
        Self {
            proxy: None,
            realms: None,
            keepalive: None,
            auth_manager: None,
            realms_api: None,
            xbl_token: None,
            user_hash: None,
            access_token: None,
            refresh_token: None,
            xuid: None,
            player_state_cache: Arc::new(BedrockPlayerStateCache::new()),
            reauth_required: false,
            renewal_lock: Arc::new(tokio::sync::Mutex::new(())),
            login_cancel_tx: None,
            proxy_target_host: None,
            proxy_target_port: None,
            proxy_listen_port: None,
            proxy_started_at: None,
            active_realm_id: None,
            active_realm_name: None,
            active_connection: None,
        }
    }

    /// Installs a signed-in Xbox session.
    ///
    /// Takes a ready `AuthManager` rather than the means to build one. Building it needs a
    /// `tauri::AppHandle`, and naming that type anywhere reachable from this struct links
    /// Tauri's window drop glue into the integration test binary, which then aborts on load.
    pub fn apply_auth(
        &mut self,
        auth_manager: Arc<AuthManager>,
        api: RealmsApi,
        xbl_token: String,
        user_hash: String,
        access_token: String,
        refresh_token: Option<String>,
        xuid: String,
    ) {
        self.auth_manager = Some(auth_manager);
        self.realms_api = Some(api);
        self.xbl_token = Some(xbl_token);
        self.user_hash = Some(user_hash);
        self.access_token = Some(access_token);
        self.refresh_token = refresh_token;
        self.xuid = Some(xuid);
        self.reauth_required = false;
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_manager.is_some()
    }

    /// Drops the Xbox Live session this process holds.
    ///
    /// Clears every field `apply_auth` sets. `BedrockState` is managed once at startup, so it
    /// outlives any one sign-in; whatever is left here belongs to whoever signed in last.
    pub fn clear_auth(&mut self) {
        self.auth_manager = None;
        self.realms_api = None;
        self.xbl_token = None;
        self.user_hash = None;
        self.access_token = None;
        self.refresh_token = None;
        self.xuid = None;
        self.reauth_required = false;
    }

    pub async fn start_keepalive(
        &mut self,
        app_state: &AppState,
        listen_port: u16,
        network_interface: &str,
    ) -> Result<(), String> {
        let xuid = self
            .xuid
            .as_ref()
            .ok_or_else(|| "XUID required for transfer keepalive".to_string())?
            .clone();

        let api = app_state
            .get_api_client()
            .map_err(|e| format!("BVC server connection required: {}", e))?;

        let server_url = api.endpoint().to_string();
        let client = api.get_reqwest_client();

        let mut keepalive = TransferKeepAlive::new(
            server_url,
            network_interface.to_string(),
            listen_port,
            client,
        );
        keepalive.start().await.map_err(|e| e.to_string())?;
        self.keepalive = Some(keepalive);
        Ok(())
    }

    pub async fn stop_keepalive(&mut self) {
        if let Some(ref mut keepalive) = self.keepalive {
            let _ = keepalive.stop().await;
        }
        self.keepalive = None;
    }
}
