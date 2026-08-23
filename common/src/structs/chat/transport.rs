use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::bedrock::AddonMode;

/// Which component carries a line composed in the app.
///
/// Chosen from the world's declared addon mode and nothing else. Whether that addon is
/// answering right now is a separate question, answered by delivery itself: a mode is a
/// statement about which component owns chat, and it does not stop being true because a
/// server restarted or briefly went away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ChatTransport {
    /// Over QUIC to the BVC server, which holds the world addon's own channel.
    Server,
    /// Injected into the game by this client's proxy, which owns chat on a no-net world.
    ProxyInjection,
}

impl ChatTransport {
    /// `None` is the no-proxy case: the app is a plain client of the BVC server, and the
    /// server carries chat. The proxy injector is chosen only where the local proxy genuinely
    /// owns the world's chat, because on every other path its queue has no consumer.
    pub fn for_mode(addon_mode: Option<AddonMode>) -> Self {
        match addon_mode {
            Some(mode) if !mode.relays_only() => Self::ProxyInjection,
            _ => Self::Server,
        }
    }
}
