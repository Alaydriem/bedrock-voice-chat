use common::structs::group_key::SocketId;

/// A Welcome that arrived before the joiner said the codes match. Held unread until then.
pub struct HeldWelcome {
    pub from: SocketId,
    pub check_id: [u8; 16],
    pub welcome: Vec<u8>,
}
