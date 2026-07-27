use std::sync::Arc;

use common::bedrock_protocol::{AuthManager, RealmsApi};
use common::traits::StreamTrait;
use tokio::sync::watch;

use crate::bedrock::BedrockAuthService;
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
    pub login_cancel_tx: Option<watch::Sender<bool>>,
    pub proxy_target_host: Option<String>,
    pub proxy_target_port: Option<u16>,
    pub proxy_listen_port: Option<u16>,
    pub active_realm_id: Option<u64>,
    pub active_realm_name: Option<String>,
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
            login_cancel_tx: None,
            proxy_target_host: None,
            proxy_target_port: None,
            proxy_listen_port: None,
            active_realm_id: None,
            active_realm_name: None,
        }
    }

    pub fn apply_auth(
        &mut self,
        api: RealmsApi,
        xbl_token: String,
        user_hash: String,
        access_token: String,
        refresh_token: Option<String>,
        xuid: String,
    ) {
        let auth_manager =
            BedrockAuthService::new().build_auth_manager(refresh_token.as_deref(), &xuid);

        self.auth_manager = Some(auth_manager);
        self.realms_api = Some(api);
        self.xbl_token = Some(xbl_token);
        self.user_hash = Some(user_hash);
        self.access_token = Some(access_token);
        self.refresh_token = refresh_token;
        self.xuid = Some(xuid);
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
            xuid,
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
