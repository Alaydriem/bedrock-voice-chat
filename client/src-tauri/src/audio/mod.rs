use common::structs::packet::QuicNetworkPacket;

pub(crate) mod device;
pub(crate) mod events;
pub(crate) mod listeners;

mod stream;

pub(crate) use stream::AudioStreamManager;

#[derive(Debug, Clone)]
pub(crate) struct AudioPacket {
    pub data: QuicNetworkPacket,
}

#[cfg(test)]
mod tests;
