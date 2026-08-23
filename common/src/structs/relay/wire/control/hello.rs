use serde::{Deserialize, Serialize};

use super::super::version::WireVersion;

// The dialer's opening frame.
//
// Offers every version this build speaks and lets the acceptor choose, so the
// choice is made once, by the side that also holds the grant.
//
// `worlds` is what the dialer hosts, not what it is permitted. Which worlds a
// peer serves is a fact about its own deployment that only it can know, so it
// is declared here rather than copied into the acceptor's config by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub versions: Vec<WireVersion>,
    pub worlds: Vec<String>,
}
