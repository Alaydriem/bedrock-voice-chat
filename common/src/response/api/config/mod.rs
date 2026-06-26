pub mod age;
pub mod bedrock;
mod check;

pub use age::ApiConfigAge;
pub use bedrock::ApiConfigBedrock;
pub use check::ApiConfigCheckResponse;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::spatial_audio_config::SpatialAudioConfig;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigResponse {
    pub status: String,
    pub client_id: String,
    pub protocol_version: String,
    pub quic_port: u32,
    #[serde(default)]
    pub spatial_audio: SpatialAudioConfig,
    #[serde(default)]
    pub bedrock: ApiConfigBedrock,
    #[serde(default)]
    pub age: ApiConfigAge,
}
