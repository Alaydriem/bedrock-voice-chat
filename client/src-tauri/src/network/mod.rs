use common::structs::packet::QuicNetworkPacket;

mod stream;

use serde::{Deserialize, Serialize};

pub(crate) use stream::NetworkStreamManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NetworkPacket {
    pub data: QuicNetworkPacket,
}
