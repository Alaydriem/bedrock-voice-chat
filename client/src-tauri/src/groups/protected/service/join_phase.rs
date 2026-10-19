use std::time::Instant;

use common::structs::group_key::SocketId;

use crate::groups::protected::{JoinerAwaitingWelcome, JoinerSession, PendingJoin};

/// Where a joiner is in admission.
pub enum JoinPhase {
    /// Socket not yet open: no socket id to start from.
    Waiting,
    AwaitingReply {
        session: JoinerSession,
        pending: PendingJoin,
        key_package: Vec<u8>,
        sent_at: Instant,
    },
    AwaitingWelcome {
        awaiting: JoinerAwaitingWelcome,
        pending: PendingJoin,
        admitter: SocketId,
        sent_at: Instant,
    },
}
