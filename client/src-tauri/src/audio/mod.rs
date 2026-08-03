use common::structs::packet::QuicNetworkPacket;

pub(crate) mod actions;
pub mod backend;
pub(crate) mod device;
pub mod encode;
pub mod recording;
pub(crate) mod speaker_test;
pub(crate) mod types;

pub(crate) mod stream;

pub use actions::AudioActionsManager;
pub use backend::AudioBackend;
pub(crate) use recording::RecordingManager;
pub use speaker_test::Chime;
pub(crate) use speaker_test::SpeakerTest;
pub(crate) use stream::AudioStreamManager;

#[derive(Debug, Clone)]
pub(crate) struct AudioPacket {
    pub data: QuicNetworkPacket,
}

#[cfg(test)]
mod tests;
