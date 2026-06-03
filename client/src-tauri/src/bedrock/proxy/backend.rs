use std::sync::Arc;

use common::bedrock_protocol::{AuthInfo, AuthManager, RealmConfig};

pub enum Backend {
    Direct {
        target_host: String,
        target_port: u16,
        auth_manager: Arc<AuthManager>,
    },
    Realm {
        realm_config: RealmConfig,
        auth: AuthInfo,
    },
}
