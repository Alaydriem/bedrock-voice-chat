use serde::{Deserialize, Serialize};

// The linked group a voice frame belongs to, carried beside the frame on a version 2 peer link.
//
// `channel` is the BVC channel id. A receiving server accepts the claim only when the speaker is
// a member of that channel. `members_only` frames are never routed by proximity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupScope {
    pub channel: String,
    pub members_only: bool,
}
