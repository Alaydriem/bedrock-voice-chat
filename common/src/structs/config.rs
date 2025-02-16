use ts_rs::TS;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfig {
    pub status: String,
    pub client_id: String,
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
