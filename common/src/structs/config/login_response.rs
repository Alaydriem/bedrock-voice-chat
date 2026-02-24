use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::keypair::Keypair;
use crate::structs::permission::ServerPermissions;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LoginResponse {
    pub gamerpic: String,
    pub gamertag: String,
    pub keypair: Keypair,
    pub signature: Keypair,
    pub certificate: String,
    pub certificate_key: String,
    pub certificate_ca: String,
    pub quic_connect_string: String,
    #[serde(default)]
    pub server_permissions: Option<ServerPermissions>,
}

impl LoginResponse {
    /// Create a new LoginResponse
    pub fn new(
        gamertag: String,
        gamerpic: String,
        keypair: Keypair,
        signature: Keypair,
        certificate: String,
        certificate_key: String,
        certificate_ca: String,
        quic_connect_string: String,
        server_permissions: Option<ServerPermissions>,
    ) -> Self {
        Self {
            gamertag,
            gamerpic,
            keypair,
            signature,
            certificate,
            certificate_key,
            certificate_ca,
            quic_connect_string,
            server_permissions,
        }
    }
}
