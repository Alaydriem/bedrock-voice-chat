use serde::{Deserialize, Serialize};

use super::super::version::WireVersion;
use crate::structs::relay::capability::Capability;

// The acceptor's answer: the version chosen, and the scope the dialer holds.
//
// `worlds` is the negotiated set — what the dialer declared, narrowed by any
// filter the acceptor configured — not a list the acceptor invented. It is
// echoed rather than assumed because a dialer whose declaration was narrowed
// otherwise looks connected and healthy while the frames it sends for the
// removed worlds are dropped at the far end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Accept {
    pub version: WireVersion,
    pub worlds: Vec<String>,
    pub capabilities: Vec<Capability>,
}
