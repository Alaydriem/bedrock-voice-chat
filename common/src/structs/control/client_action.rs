use serde::{Deserialize, Serialize};

use super::ClientActionType;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct ClientAction {
    pub id: String,
    pub action: ClientActionType,
}
