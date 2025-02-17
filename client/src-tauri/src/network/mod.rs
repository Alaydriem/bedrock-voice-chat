use common::structs::packet::QuicNetworkPacket;

pub(crate) mod events;
pub(crate) mod listeners;
mod stream;

pub(crate) use stream::NetworkStreamManager;

#[derive(Debug, Clone)]
pub(crate) struct NetworkPacket {
    pub data: QuicNetworkPacket,
}
