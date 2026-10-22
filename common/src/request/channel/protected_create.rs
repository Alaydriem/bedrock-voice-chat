use serde::{Deserialize, Serialize};

/// Body of `POST /api/channel/protected`. Carries no password: the server never receives one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct ProtectedChannelCreateRequest {
    /// Empty asks the server to generate a name.
    pub name: String,
}
