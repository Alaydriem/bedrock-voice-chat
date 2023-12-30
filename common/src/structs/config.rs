use ts_rs::TS;

use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../client/src/js/bindings/")]
pub struct ApiConfig {
    pub status: String,
    pub client_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]

#[ts(export, export_to = "./../client/src/js/bindings/")]
pub struct LoginRequest {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]

#[ts(export, export_to = "./../client/src/js/bindings/")]
pub struct LoginResponse {
    pub gamerpic: String,
    pub gamertag: String,
    pub cert: String,
    pub key: String,
}

#[derive(Clone, Serialize, Deserialize, TS)]

#[ts(export, export_to = "./../client/src/js/bindings/")]
pub enum StreamType {
    InputStream,
    OutputStream,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../client/src/js/bindings/")]
pub struct MicrosoftAuthCodeAndUrlResponse {
    pub url: String,
    pub state: String,
}
