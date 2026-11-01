use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the client's Rust-side position socket tells the webview. The webview cannot validate
/// a play-zone certificate itself, so the socket lives in Rust and these are its events.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(tag = "kind")]
pub enum PositionSocketEvent {
    Open,
    Message { data: String },
    Closed,
    Error { reason: String },
}
