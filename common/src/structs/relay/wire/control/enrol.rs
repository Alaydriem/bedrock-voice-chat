use serde::{Deserialize, Serialize};

use super::super::version::WireVersion;

// The dialer's opening frame when it is redeeming a pairing code.
//
// Carries everything `Hello` does, plus the code. A separate variant rather than a field
// added to `Hello`: adding a field would change that variant's encoding and mis-decode
// against every bridge already deployed.
//
// `code` is the plaintext an operator typed. It crosses the wire once, on a connection
// already authenticated by the peer's key, and is never stored by the dialer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enrol {
    pub versions: Vec<WireVersion>,
    pub worlds: Vec<String>,
    pub code: String,
}
