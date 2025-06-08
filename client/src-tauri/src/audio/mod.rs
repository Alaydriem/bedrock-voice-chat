use common::structs::packet::QuicNetworkPacket;

pub(crate) mod device;
pub(crate) mod types;

mod stream;

pub(crate) use stream::AudioStreamManager;

#[derive(Debug, Clone)]
pub(crate) struct AudioPacket {
    pub data: QuicNetworkPacket,
}


#[cfg(test)]
mod tests;
