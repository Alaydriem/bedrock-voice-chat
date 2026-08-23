use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Proof that the push channel is still carrying, sent whether or not anything happened.
///
/// The channel is otherwise silent by design: levels are published on change and never re-send
/// silence, and metrics and health only arrive while a session is connected. So a quiet channel
/// and a dead one look identical from the page, and the page is where that distinction has to be
/// drawn — a socket that Android suspended reports `readyState` 1 and delivers nothing, with no
/// close event to react to.
///
/// Carries no data. Its arrival is the whole payload, which is why no consumer subscribes to it:
/// `EventChannel` times every frame it receives and this is the one that always comes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct KeepalivePush {
    #[serde(rename = "type")]
    pub kind: String,
}

impl KeepalivePush {
    pub const KIND: &'static str = "keepalive";

    pub fn new() -> Self {
        Self {
            kind: Self::KIND.to_string(),
        }
    }
}

impl Default for KeepalivePush {
    fn default() -> Self {
        Self::new()
    }
}
