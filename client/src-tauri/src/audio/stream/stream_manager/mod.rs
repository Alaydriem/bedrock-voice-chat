mod audio_sink;
pub(crate) mod device_lease;
#[cfg(feature = "e2e")]
pub(crate) mod frame_clock;
mod input;
mod input_core;
pub(crate) mod job_set;
mod mono_to_panned;
pub mod output;
mod resampler;
pub(crate) mod sink;
mod sink_manager;
pub(crate) mod source;

use anyhow::anyhow;
use common::structs::audio::StreamEvent;
use std::sync::Arc;

use crate::audio::AudioDevice;

pub(crate) use audio_sink::AudioSinkType;
pub(crate) use device_lease::DeviceLease;
pub(crate) use job_set::JobSet;
pub(crate) use common::traits::StreamTrait;
pub(crate) use input::InputStream;
pub(crate) use output::OutputStream;
pub(crate) use sink::AudioOutputSink;
pub(crate) use source::AudioInputSource;

mod audio_frame;
mod audio_frame_data;
mod mute_flags;
mod noise_gate_flags;

pub(crate) use audio_frame::AudioFrame;
pub(crate) use audio_frame_data::AudioFrameData;
pub(crate) use mute_flags::MuteFlags;
pub(crate) use noise_gate_flags::NoiseGateFlags;

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
