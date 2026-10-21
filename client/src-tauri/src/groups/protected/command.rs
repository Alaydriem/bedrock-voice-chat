use common::structs::group_key::SocketId;
use tokio::sync::oneshot;

/// A person's decision, carried from a Tauri command to the runner that owns the group's service.
pub enum ProtectedGroupCommand {
    /// The member pressed "Check code" on a joiner's card.
    Check {
        joiner: SocketId,
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
    /// The member pressed Admit or Deny on the card showing `check_id`.
    Decide {
        joiner: SocketId,
        check_id: [u8; 16],
        allow: bool,
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
    /// The joiner pressed "It matches" or "No match".
    Confirm {
        check_id: [u8; 16],
        matches: bool,
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
}
