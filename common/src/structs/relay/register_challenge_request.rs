use serde::{Deserialize, Serialize};

use super::relay_endpoint::RelayEndpoint;

// First leg of the endpoint-control-proven registration. A
// registrant asks the relay for a challenge bound to the endpoint it intends to
// register. The relay issues a nonce (`RegisterChallengeResponse`) and verifies
// the registrant actually controls that endpoint by fetching the nonce back from
// it before it will accept a `register` for the endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct RegisterChallengeRequest {
    pub endpoint: RelayEndpoint,
}
