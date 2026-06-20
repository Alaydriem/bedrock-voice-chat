use serde::{Deserialize, Serialize};

// Asker → minter: "issue me a peer code for `hashed_world`, bound to my endpoint."
// The minter mints a single-use code recipient-bound to `{asker_host}:{asker_port}`
// and injects it into the realm via its own client; only a live member of the
// world ever sees it, and only the bound asker may redeem it.
//
// `asker_public_key` is the asker's X25519 public key (32 bytes). The minter
// SEALS the minted code to it (libsodium sealed box) before injecting it, so the
// code travels through the realm as ciphertext only the asker can open — an
// observer who intercepts it cannot redeem it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct OfferRequest {
    pub hashed_world: String,
    pub asker_host: String,
    pub asker_port: u16,
    pub asker_public_key: Vec<u8>,
}
