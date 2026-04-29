use common::Game;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityMetadata {
    pub gamertag: String,
    pub game: Game,
    pub server_url: String,
    pub cert_not_after: Option<i64>,
}
