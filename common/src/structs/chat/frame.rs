use serde::{Deserialize, Serialize};

/// The wire format on `/api/websocket/chat`.
///
/// Text frames, JSON, tagged on `t`. Lives in `common` so the server and the hand-written
/// Kotlin and TypeScript encoders cannot drift apart without something failing to compile or
/// failing a test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum ChatFrame {
    /// Always the first frame. The worlds it names are a property of the connection, so no
    /// later frame carries one.
    ///
    /// `world` is the canonical id and supplies the picker's label. `worlds` covers the case
    /// where one chat room spans several world ids: Paper and Fabric mint a UUID per
    /// dimension, so a single server's overworld, nether and end are three ids — and chat is
    /// server-wide, not per-dimension. BDS mints one id for the whole world and omits it.
    Hello {
        world: String,
        world_name: String,
        game: String,
        #[serde(default)]
        worlds: Vec<String>,
    },

    /// A line a player typed in game, mod to server.
    Chat { author: String, text: String },

    /// Something the server said rather than a person: a death, a join, a leave, a broadcast.
    ///
    /// Carries no author, which is what makes it render as a system line. On the no-net path
    /// the proxy gets these free because it sees every `TextPacket` type; a mod has to report
    /// them explicitly, and this is the frame it uses.
    Event { text: String },

    /// A line composed in the app, server to mod, for broadcasting in game.
    Say { author: String, text: String },
}
