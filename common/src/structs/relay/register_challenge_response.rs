use serde::{Deserialize, Serialize};

// The relay's response to a `RegisterChallengeRequest`. `nonce` is
// the value the registrant must serve at its claimed endpoint
// (`/relay/proof/<nonce>`) so the relay can confirm endpoint control. `token` is
// the bearer the registrant then presents to `register`; it is bound to the
// endpoint the challenge was issued for and is only honored once the relay has
// observed the nonce served back from that endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisterChallengeResponse {
    pub token: String,
    pub nonce: String,
}
