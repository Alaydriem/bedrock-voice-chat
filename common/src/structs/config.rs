use ts_rs::TS;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfig {
    pub status: String,
    pub client_id: String,
    pub protocol_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LoginRequest {
    pub code: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct Keypair {
    pub pk: Vec<u8>,
    pub sk: Vec<u8>,
}

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
        }
    }
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum StreamType {
    InputStream,
    OutputStream,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct MicrosoftAuthCodeAndUrlResponse {
    pub url: String,
    pub state: String,
}

/// Response when starting a Hytale device flow
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct HytaleDeviceFlowStartResponse {
    pub session_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u32,
    pub interval: u32,
}

/// Status of a Hytale device flow authentication
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum HytaleAuthStatus {
    Pending,
    Success,
    Expired,
    Denied,
    Error,
}

/// Response when polling Hytale device flow status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct HytaleDeviceFlowStatusResponse {
    pub status: HytaleAuthStatus,
    pub login_response: Option<LoginResponse>,
}
