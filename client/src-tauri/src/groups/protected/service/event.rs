use common::structs::group_key::SocketId;

use super::{CodeRole, LostReason};

/// What the key service tells the rest of the app. None of these carries key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedGroupEvent {
    Joined { epoch: u64, safety_code: String },
    /// Canonical identities, from the group's own MLS state.
    RosterChanged(Vec<String>),
    EpochChanged { epoch: u64, safety_code: String },
    Lost(LostReason),
    /// Member: a new join request card. `name` is the identity the server gave for the socket.
    /// `cue` says whether the join-request sound plays for it.
    Request {
        joiner: SocketId,
        name: String,
        cue: bool,
    },
    /// Member: requests beyond the cards shown.
    Waiting(usize),
    /// Either side: a code to compare, for one check.
    Code {
        role: CodeRole,
        check_id: [u8; 16],
        joiner: SocketId,
        peer_name: String,
        code: String,
    },
    /// Member: another member is checking this joiner.
    Checking { joiner: SocketId, member_name: String },
    /// Member: this check failed; someone may be interfering.
    CheckFailed { joiner: SocketId },
    /// Member: the request closed without an admission.
    RequestEnded { joiner: SocketId },
    /// Member: the joiner's Welcome went out.
    Admitted { joiner: SocketId },
}
