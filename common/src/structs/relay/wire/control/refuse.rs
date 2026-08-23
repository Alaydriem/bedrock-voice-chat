use serde::{Deserialize, Serialize};

use super::refuse_reason::RefuseReason;

// A refusal carrying its reason.
//
// Sent instead of closing bare, so the dialer can tell a configuration problem
// from a network one without an operator reading both servers' logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refuse {
    pub reason: RefuseReason,
}
