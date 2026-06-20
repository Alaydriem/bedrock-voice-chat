use bedrock_client::ClientConnection;

use crate::harness::client_proc::ClientProc;

/// One proxied player. `_downstream` is held (not dropped) for the world's
/// lifetime: tokio-raknet aborts on drop, so dropping it would tear down the
/// proxy session.
pub struct ProxyPlayer {
    pub proc: ClientProc,
    pub(super) _downstream: ClientConnection,
}
