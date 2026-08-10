use common::structs::packet::QuicNetworkPacket;

mod stream;
mod transport_verdict;

use serde::{Deserialize, Serialize};

pub(crate) use stream::NetworkStreamManager;
pub use transport_verdict::TransportVerdict;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPacket {
    pub data: QuicNetworkPacket,
}
