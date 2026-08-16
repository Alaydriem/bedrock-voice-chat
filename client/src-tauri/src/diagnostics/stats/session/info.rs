use std::time::Instant;

use common::structs::metrics::TransportKind;
use common::structs::reachability::AddressFamily;


#[derive(Debug, Clone)]
pub(super) struct SessionInfo {
    pub(super) connected_at: Instant,
    // Absent on a WebSocket session: the family is chosen inside the TLS dialler rather
    // than by a connect walk this can read it from, and reporting a guess would be worse
    // than reporting nothing.
    pub(super) family: Option<AddressFamily>,
    pub(super) port: u16,
    pub(super) transport: TransportKind,
    pub(super) server: String,
    pub(super) server_id: String,
}
