use common::structs::packet::QuicNetworkPacket;

pub(crate) mod actions;
pub mod backend;
pub(crate) mod device;
pub mod encode;
pub mod recording;
pub(crate) mod types;

pub(crate) mod stream;

pub use actions::AudioActionsManager;
pub use backend::AudioBackend;
pub(crate) use recording::RecordingManager;
pub(crate) use stream::AudioStreamManager;

#[derive(Debug, Clone)]
pub(crate) struct AudioPacket {
    pub data: QuicNetworkPacket,
}

#[cfg(test)]
mod tests;
