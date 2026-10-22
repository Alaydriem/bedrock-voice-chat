use crate::groups::protected::{JoinerRequest, PendingJoin};

use super::HeldWelcome;

/// Where a joiner is in admission.
pub enum JoinPhase {
    /// Socket not yet open: no request sent.
    Waiting,
    /// The request is out: a single-use ticket and the KeyPackage it commits to.
    Asking {
        request: JoinerRequest,
        pending: PendingJoin,
        held: Option<HeldWelcome>,
    },
}
