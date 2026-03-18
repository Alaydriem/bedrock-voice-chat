use common::structs::packet::QuicNetworkPacket;

#[derive(Debug, Clone)]
pub struct ServerInputPacket {
    pub data: QuicNetworkPacket,
}
