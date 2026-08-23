use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct WebSocketClientInfo {
    pub id: String,
    // Taken from the client's User-Agent. A client that sends none is still listed:
    // an unnamed connection is exactly the one an operator is looking for.
    pub name: String,
    pub route: String,
    pub connected_at: u64,
    pub commands: u32,
}
