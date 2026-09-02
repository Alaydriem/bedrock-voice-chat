use serde::{Deserialize, Serialize};

// The only time the secret half exists outside the caller's terminal. It is not
// recoverable afterwards: the server stores nothing but its hash. `revoked` names the
// credential a rotation retired, and is absent for a plain mint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct MintedTokenResponse {
    pub id: String,
    pub token: String,
    pub revoked: Option<String>,
}
