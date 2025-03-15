use common::structs::packet::QuicNetworkPacket;

mod stream;

use serde::{Deserialize, Serialize};
use anyhow::anyhow;

pub(crate) use stream::NetworkStreamManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NetworkPacket {
    pub data: QuicNetworkPacket,
}