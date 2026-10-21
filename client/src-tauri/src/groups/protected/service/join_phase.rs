use std::time::Instant;

use common::structs::group_key::{AdmissionMessage, SocketId};

use crate::groups::protected::{JoinerAwaitingWelcome, JoinerSession, PendingJoin};

/// Where a joiner is in admission.
pub enum JoinPhase {
    /// Socket not yet open: no socket id to start from.
    Waiting,
    AwaitingReply {
        session: JoinerSession,
        pending: PendingJoin,
        key_package: Vec<u8>,
        // Kept so a redirect can send the same init on to the admitter.
        init: AdmissionMessage,
        // At most one redirect is followed per attempt, so a relay cannot bounce a join forever.
        redirected: bool,
        sent_at: Instant,
    },
    AwaitingWelcome {
        awaiting: JoinerAwaitingWelcome,
        pending: PendingJoin,
        admitter: SocketId,
        sent_at: Instant,
    },
}
