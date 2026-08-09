use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ConnectTargetKind;

/// The world this client is connected to right now.
///
/// Rides on every state frame so a controller draws its toggle from the same push it
/// already subscribes to, rather than polling a second endpoint that would go stale the
/// moment the user disconnects from the app itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActiveConnection {
    pub id: String,
    pub name: String,
    pub kind: ConnectTargetKind,
}
