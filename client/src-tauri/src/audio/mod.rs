use common::structs::packet::QuicNetworkPacket;

pub(crate) mod actions;
pub(crate) mod device;
pub mod recording;
pub(crate) mod types;

mod stream;

pub(crate) use actions::AudioActionsManager;
pub(crate) use recording::RecordingManager;
pub(crate) use stream::AudioStreamManager;

#[derive(Debug, Clone)]
pub(crate) struct AudioPacket {
    pub data: QuicNetworkPacket,
}

#[cfg(test)]
mod tests;
