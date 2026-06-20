use bedrock_client::ClientConnection;

use crate::harness::client_proc::ClientProc;

pub(super) struct RelayPlayer {
    pub(super) proc: ClientProc,
    // Held alive for the world's lifetime (tokio-raknet aborts on drop).
    pub(super) _downstream: ClientConnection,
}
