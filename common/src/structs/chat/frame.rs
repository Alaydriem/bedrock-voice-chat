use serde::{Deserialize, Serialize};

/// The wire format on `/api/websocket/chat`.
///
/// Text frames, JSON, tagged on `t`. Lives in `common` so the server and the hand-written
/// Kotlin and TypeScript encoders cannot drift apart without something failing to compile or
/// failing a test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "openapi",
    schemars(
        description = "One chat message. The t field says which kind: hello, chat, event or say."
    )
)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum ChatFrame {
    /// Always the first frame. The worlds it names are a property of the connection, so no
    /// later frame carries one.
    ///
    /// `world` is the canonical id and supplies the picker's label. `worlds` covers the case
    /// where one chat room spans several world ids: Paper and Fabric mint a UUID per
    /// dimension, so a single server's overworld, nether and end are three ids — and chat is
    /// server-wide, not per-dimension. BDS mints one id for the whole world and omits it.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "Game to server. Send this first, once. It says which world the socket carries chat for."
        )
    )]
    Hello {
        #[cfg_attr(
            feature = "openapi",
            schemars(
                description = "The world ID. Use the same ID your Addon or mod reports player positions with."
            )
        )]
        world: String,
        #[cfg_attr(
            feature = "openapi",
            schemars(description = "The world name shown to players in the app.")
        )]
        world_name: String,
        #[cfg_attr(
            feature = "openapi",
            schemars(description = "The game. Send minecraft.")
        )]
        game: String,
        #[cfg_attr(
            feature = "openapi",
            schemars(
                description = "Optional. More world IDs that share this chat, for a server that gives each dimension its own ID. Leave it out when there is only one."
            )
        )]
        #[serde(default)]
        worlds: Vec<String>,
    },

    /// A line a player typed in game, mod to server.
    #[cfg_attr(
        feature = "openapi",
        schemars(description = "Game to server. A message a player typed in game.")
    )]
    Chat {
        #[cfg_attr(
            feature = "openapi",
            schemars(description = "The player who typed it.")
        )]
        author: String,
        #[cfg_attr(feature = "openapi", schemars(description = "The message."))]
        text: String,
    },

    /// Something the server said rather than a person: a death, a join, a leave, a broadcast.
    ///
    /// Carries no author, which is what makes it render as a system line. On the no-net path
    /// the proxy gets these free because it sees every `TextPacket` type; a mod has to report
    /// them explicitly, and this is the frame it uses.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "Game to server. A line from the game itself, such as a death, a join or a broadcast. The app shows it without an author."
        )
    )]
    Event {
        #[cfg_attr(feature = "openapi", schemars(description = "The message."))]
        text: String,
    },

    /// A line composed in the app, server to mod, for broadcasting in game.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "Server to game. A message a player typed in the app. Show it in game chat."
        )
    )]
    Say {
        #[cfg_attr(
            feature = "openapi",
            schemars(description = "The player who typed it in the app.")
        )]
        author: String,
        #[cfg_attr(feature = "openapi", schemars(description = "The message."))]
        text: String,
    },
}
