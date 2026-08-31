use serde::{Deserialize, Serialize};

// A freshly minted pairing code on its way to the operator who asked for it.
//
// The plaintext travels here and is stored nowhere. Nothing may log this type: a code in
// a log file is a credential in a log file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PairingCodeResponse {
    pub code: String,
    pub label: String,
    pub expires_in_secs: u64,
}
