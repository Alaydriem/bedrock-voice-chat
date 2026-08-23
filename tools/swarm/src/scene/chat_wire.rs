use serde::Serialize;

/// The wire format on `/api/websocket/chat`.
///
/// Restated here rather than imported: this is a standalone workspace and does not depend on
/// `common`, where `ChatFrame` is defined. Drift is loud rather than silent — the server logs
/// "chat socket sent an undecodable frame" and no line arrives — which is why restating four
/// small frames is preferable to pulling the client's whole dependency tree in behind them.
///
/// Serialize only. `say` travels the other way and this end never reads it.
#[derive(Serialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum ChatWireFrame {
    /// Always first. The world it names is a property of the connection, so no later frame
    /// carries one.
    Hello {
        world: String,
        world_name: String,
        game: String,
        worlds: Vec<String>,
    },

    /// A line a player typed in game.
    Chat { author: String, text: String },

    /// Something the server said. Carries no author, which is what makes it render as a
    /// system line rather than as a person talking.
    Event { text: String },
}
