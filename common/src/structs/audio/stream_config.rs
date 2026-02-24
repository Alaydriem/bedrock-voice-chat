use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[cfg(feature = "audio")]
use rodio::cpal::{ChannelCount, SampleFormat, SupportedStreamConfigRange};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct StreamConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub sample_format: String,
    pub buffer_size_min: u32,
    pub buffer_size_max: u32,
}

/// Maps the Stream Config to a rodio::cpal::SupportedStreamConfigRange
#[cfg(feature = "audio")]
impl Into<SupportedStreamConfigRange> for StreamConfig {
    fn into(self) -> SupportedStreamConfigRange {
        SupportedStreamConfigRange::new(
            self.channels as ChannelCount,
            self.sample_rate,
            self.sample_rate,
            rodio::cpal::SupportedBufferSize::Range {
                min: self.buffer_size_min,
                max: self.buffer_size_max,
            },
            match self.sample_format.as_str() {
                "f32" => SampleFormat::F32,
                "i32" => SampleFormat::I32,
                "i16" => SampleFormat::I16,
                _ => SampleFormat::F32, // default case
            },
        )
    }
}

/// Maps the Stream Config to a rodio::cpal::StreamConfig
#[cfg(feature = "audio")]
impl Into<rodio::cpal::StreamConfig> for StreamConfig {
    fn into(self) -> rodio::cpal::StreamConfig {
        rodio::cpal::StreamConfig {
            channels: self.channels as ChannelCount,
            sample_rate: self.sample_rate,
            buffer_size: rodio::cpal::BufferSize::Fixed(crate::consts::audio::BUFFER_SIZE),
        }
    }
}

/// Maps the Stream Config to a rodio::cpal::SupportedStreamConfig
#[cfg(feature = "audio")]
impl Into<rodio::cpal::SupportedStreamConfig> for StreamConfig {
    fn into(self) -> rodio::cpal::SupportedStreamConfig {
        rodio::cpal::SupportedStreamConfig::new(
            self.channels as ChannelCount,
            self.sample_rate,
            rodio::cpal::SupportedBufferSize::Range {
                min: self.buffer_size_min,
                max: self.buffer_size_max,
            },
            match self.sample_format.as_str() {
                "f32" => SampleFormat::F32,
                "i32" => SampleFormat::I32,
                "i16" => SampleFormat::I16,
                _ => SampleFormat::F32, // default case
            },
        )
    }
}
