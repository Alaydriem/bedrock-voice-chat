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
// Re-exported rather than opening `stream`: the watchdog's decision rule is a behavioural
// contract worth testing on its own, and the rest of that module is not.
pub use stream::capture_watchdog::{CaptureVerdict, CaptureWatchdog};
// Re-exported for the same reason as the watchdog above: how many times a failing stream is
// rebuilt, and when to give up, is a decision rule worth testing without an audio device.
pub use stream::rebuild_breaker::{RebuildBreaker, RebuildVerdict};
pub use stream::capture_availability::CaptureAvailability;
// Same reason: what makes a level worth a webview message, and how a measured RMS becomes a
// step, are both decision rules worth testing without an audio device or a webview.
pub use stream::level_bus::{LevelBus, LevelEmitPolicy, LoudnessTracker};
pub use stream::stream_manager::device_lease::DeviceLease;
pub use stream::stream_manager::job_set::JobSet;
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
