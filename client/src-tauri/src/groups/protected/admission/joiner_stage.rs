use std::time::Instant;

use common::structs::group_key::SocketId;

use super::ApprovalCode;

/// Where a join request is. A request moves forward only: one reveal, then an end.
pub enum JoinerStage {
    Requested,
    Revealed {
        check_id: [u8; 16],
        checker: SocketId,
        signature_key: Vec<u8>,
        mls_group_id: Vec<u8>,
        code: ApprovalCode,
        revealed_at: Instant,
        confirmed: bool,
    },
    Ended,
}
