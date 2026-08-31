pub mod age;
pub mod bedrock;
pub mod capacity;
mod check;
pub mod chat;
mod compatibility;
pub mod recording;

pub use age::ApiConfigAge;
pub use bedrock::{ApiConfigBedrock, ApiConfigBedrockServer};
pub use capacity::ApiConfigCapacity;
pub use check::ApiConfigCheckResponse;
pub use chat::ApiConfigChat;
pub use compatibility::ProtocolCompatibility;
pub use recording::ApiConfigRecording;

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
    // Every public UDP port that reaches this server's QUIC listener, in the
    // operator's preferred order. Independent of the bound port: a fronting proxy
    // and a direct publish can both deliver to the same socket. Empty from a
    // server that predates this field, in which case `quic_port` stands alone.
    #[serde(default)]
    pub quic_ports: Vec<u32>,
    // Whether this server carries voice over TLS WebSocket as well as QUIC. False from a
    // server that predates the transport, which is exactly what it has to mean: a client
    // whose UDP is blocked has no path to such a server, and probing for one would report a
    // fallback that does not exist.
    #[serde(default)]
    pub voice_websocket: bool,
    #[serde(default)]
    pub spatial_audio: SpatialAudioConfig,
    #[serde(default)]
    pub bedrock: ApiConfigBedrock,
    #[serde(default)]
    pub age: ApiConfigAge,
    #[serde(default)]
    pub recording: ApiConfigRecording,
    #[serde(default)]
    pub chat: ApiConfigChat,
    #[serde(default)]
    pub capacity: ApiConfigCapacity,

    // The peer endpoint a Simple Voice Chat bridge dials, or `None` on a server with
    // peering turned off.
    //
    // Not a credential: a bridge still redeems a pairing code before it is authorized, and
    // possession of this grants nothing. Absent rather than empty when peering is off, so a
    // caller cannot read a blank string as a reachable endpoint.
    #[serde(default)]
    pub peer_link: Option<String>,
}
