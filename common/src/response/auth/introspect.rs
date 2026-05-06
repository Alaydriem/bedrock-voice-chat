use serde::{Deserialize, Serialize};

use crate::structs::permission::Permission;
use crate::Game;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct IntrospectResponse {
    pub gamertag: String,
    pub game: Game,
    pub cert_not_after: Option<i64>,
    pub permissions: Vec<Permission>,
}
