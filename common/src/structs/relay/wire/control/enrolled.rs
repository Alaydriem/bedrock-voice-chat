use serde::{Deserialize, Serialize};

use super::super::version::WireVersion;
use crate::structs::relay::capability::Capability;

// The acceptor's answer to a redeemed code: the scope the dialer now holds.
//
// Shaped like `Accept` because it reports the same thing. Distinct because a dialer has
// to tell "you are now paired" from "you were already authorized", and folding them
// would leave a bridge unable to report which happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enrolled {
    pub version: WireVersion,
    pub worlds: Vec<String>,
    pub capabilities: Vec<Capability>,
}
