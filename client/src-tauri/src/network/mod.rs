use common::structs::packet::QuicNetworkPacket;

mod stream;

pub(crate) use stream::NetworkStreamManager;

#[derive(Debug, Clone)]
pub(crate) struct NetworkPacket {
    pub data: QuicNetworkPacket,
}
