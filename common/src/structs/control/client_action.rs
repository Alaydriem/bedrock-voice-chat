use serde::{Deserialize, Serialize};

use super::ClientActionType;
use crate::Game;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct ClientAction {
    pub id: String,
    /// The game the actor is playing, mirroring `GameDataCollection.game` on the position
    /// request.
    ///
    /// Optional because the BDS and Java encoders build this JSON by hand and do not consume
    /// this crate: a mod that predates the field must still be understood rather than
    /// rejected. `actor_key` is where the absence is resolved.
    #[serde(default)]
    pub game: Option<Game>,
    pub action: ClientActionType,
}

impl ClientAction {
    /// The actor's canonical identity, `game:gamertag` — the key channel membership, the
    /// connection registry and the position cache are all indexed on.
    ///
    /// A missing `game` resolves to Minecraft, which is what every caller hardcoded before
    /// the field existed.
    pub fn actor_key(&self) -> String {
        self.game
            .clone()
            .unwrap_or(Game::Minecraft)
            .membership_key(&self.id)
    }
}
