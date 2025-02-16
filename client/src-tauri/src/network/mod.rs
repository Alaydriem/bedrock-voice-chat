use common::structs::packet::QuicNetworkPacket;

pub(crate) mod commands;

#[derive(Debug, Clone)]
pub(crate) struct NetworkPacket {
    pub data: QuicNetworkPacket,
}
