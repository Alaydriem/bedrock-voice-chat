use serde::{Deserialize, Serialize};

use crate::Game;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct CreateUserRequest {
    pub gamertag: String,
    pub game: Game,
}
