use std::sync::Arc;

use bedrock_protocol::{AuthManager, RealmsApi};
use tokio::sync::watch;

use crate::bedrock::keepalive::TransferKeepalive;
use crate::bedrock::position_cache::BedrockPositionCache;
use crate::bedrock::proxy::ProxyConnectManager;
use crate::bedrock::realms::RealmsConnectManager;

pub struct BedrockState {
    pub proxy: Option<ProxyConnectManager>,
    pub realms: Option<RealmsConnectManager>,
    pub keepalive: Option<TransferKeepalive>,
    pub auth_manager: Option<Arc<AuthManager>>,
    pub realms_api: Option<RealmsApi>,
    pub xbl_token: Option<String>,
    pub user_hash: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub xuid: Option<String>,
    pub position_cache: Arc<BedrockPositionCache>,
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
            position_cache: Arc::new(BedrockPositionCache::new()),
            login_cancel_tx: None,
            proxy_target_host: None,
            proxy_target_port: None,
            proxy_listen_port: None,
            active_realm_id: None,
            active_realm_name: None,
        }
    }
}
