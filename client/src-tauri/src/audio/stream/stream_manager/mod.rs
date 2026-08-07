mod audio_sink;
pub(crate) mod device_lease;
#[cfg(feature = "e2e")]
pub(crate) mod frame_clock;
mod input;
mod input_core;
pub(crate) mod job_set;
mod mono_to_panned;
mod output;
mod resampler;
pub(crate) mod sink;
mod sink_manager;
pub(crate) mod source;

use anyhow::anyhow;
use common::structs::audio::StreamEvent;
use std::sync::Arc;

use crate::audio::types::AudioDevice;

pub(crate) use audio_sink::AudioSinkType;
pub(crate) use device_lease::DeviceLease;
pub(crate) use job_set::JobSet;
pub(crate) use common::traits::StreamTrait;
pub(crate) use input::InputStream;
pub(crate) use output::OutputStream;
pub(crate) use sink::AudioOutputSink;
pub(crate) use source::AudioInputSource;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum AudioFrame {
    F32(AudioFrameData<f32>),
    I32(AudioFrameData<i32>),
    I16(AudioFrameData<i16>),
}

impl AudioFrame {
    pub fn f32(self) -> Option<AudioFrameData<f32>> {
        if let AudioFrame::F32(f) = self {
            return Some(f);
        }

        None
    }

    #[allow(unused)]
    pub fn i32(self) -> Option<AudioFrameData<i32>> {
        if let AudioFrame::I32(f) = self {
            return Some(f);
        }

        None
    }

    #[allow(unused)]
    pub fn i16(self) -> Option<AudioFrameData<i16>> {
        if let AudioFrame::I16(f) = self {
            return Some(f);
        }

        None
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AudioFrameData<T> {
    pub pcm: Vec<T>,
    /// Timestamp when audio was captured at the CPAL callback, for accurate recording timecode
    pub captured_at_ms: u64,
}

pub(crate) enum StreamTraitType {
    Input(InputStream),
    Output(OutputStream),
}

impl common::traits::StreamTrait for StreamTraitType {
    fn is_stopped(&self) -> bool {
        match self {
            Self::Input(stream) => stream.is_stopped(),
            Self::Output(stream) => stream.is_stopped(),
        }
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.stop().await,
            Self::Output(stream) => stream.stop().await,
        }
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.start().await,
            Self::Output(stream) => stream.start().await,
        }
    }

    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.metadata(key, value).await,
            Self::Output(stream) => stream.metadata(key, value).await,
        }
    }
}

impl StreamTraitType {
    #[allow(unused)]
    pub fn get_device(&self) -> Option<AudioDevice> {
        match self {
            Self::Input(stream) => stream.device.clone(),
            Self::Output(stream) => stream.device.clone(),
        }
    }

    pub fn get_metadata(&self) -> Arc<moka::future::Cache<String, String>> {
        match self {
            Self::Input(stream) => stream.metadata.clone(),
            Self::Output(stream) => stream.metadata.clone(),
        }
    }

    pub fn toggle(&self, event: StreamEvent) {
        match self {
            Self::Input(stream) => stream.toggle(event),
            Self::Output(stream) => stream.toggle(event),
        }
    }

    pub fn mute_status(&self) -> bool {
        match self {
            Self::Input(stream) => stream.mute_status(),
            Self::Output(stream) => stream.mute_status(),
        }
    }

    /// Capture and meter with nothing attached to the network. Input only — there is no
    /// output equivalent, because a level with no session behind it is a property of a
    /// microphone and an output stream has nothing to measure.
    pub async fn start_metering(&mut self) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.start_metering().await,
            Self::Output(_) => Err(anyhow!("output streams cannot be metered")),
        }
    }

    pub fn reset_stats(&self) {
        match self {
            Self::Input(stream) => stream.reset_stats(),
            Self::Output(_) => {}
        }
    }

    /// Whether a session capture stream is supposed to be delivering frames right now.
    ///
    /// Always false for an output stream: there is no capture counter behind it, so absence
    /// there says nothing.
    pub fn capture_expected(&self) -> bool {
        match self {
            Self::Input(stream) => stream.capture_expected(),
            Self::Output(_) => false,
        }
    }
}

// Live mute state, readable without locking the audio manager. Both flags are process-global
// already; this exists so a diagnostic can observe them without reaching into the private stream
// modules.
pub(crate) struct MuteFlags;

impl MuteFlags {
    pub(crate) fn input_muted() -> bool {
        input::MUTE_INPUT_STREAM.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn output_muted() -> bool {
        output::MUTE_OUTPUT_STREAM.load(std::sync::atomic::Ordering::Relaxed)
    }

    // Test-only setters. The flags are process-global and normally moved by a keybind, the UI, an
    // in-game command or a WebSocket client; without these a test can only observe the default and
    // so cannot tell a wired field from a hardcoded one.
    #[cfg(any(test, feature = "e2e"))]
    pub(crate) fn set_input_muted(muted: bool) {
        input::MUTE_INPUT_STREAM.store(muted, std::sync::atomic::Ordering::Relaxed);
    }

    #[cfg(any(test, feature = "e2e"))]
    pub(crate) fn set_output_muted(muted: bool) {
        output::MUTE_OUTPUT_STREAM.store(muted, std::sync::atomic::Ordering::Relaxed);
    }
}

// Whether the noise gate is bound to the capture path, readable without locking the audio
// manager. Exists for the same reason as `MuteFlags`: a diagnostic has to observe the flag
// the audio path actually reads, not the copy the settings screen holds. The two disagreeing
// is precisely the fault this reports on.
pub(crate) struct NoiseGateFlags;

impl NoiseGateFlags {
    pub(crate) fn enabled() -> bool {
        input::USE_NOISE_GATE.load(std::sync::atomic::Ordering::Relaxed)
    }

    // Test-only setter, for the same reason as the mute ones: without it a test can only
    // observe the default and cannot tell a wired field from a hardcoded one.
    #[cfg(any(test, feature = "e2e"))]
    pub(crate) fn set_enabled(enabled: bool) {
        input::USE_NOISE_GATE.store(enabled, std::sync::atomic::Ordering::Relaxed);
    }
}

pub(crate) fn output_mute_state() -> bool {
    output::MUTE_OUTPUT_STREAM.load(std::sync::atomic::Ordering::Relaxed)
}
