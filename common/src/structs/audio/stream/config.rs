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

#[cfg(feature = "audio")]
impl StreamConfig {
    pub fn best_sample_rate(config: &SupportedStreamConfigRange) -> Option<u32> {
        for rate in crate::consts::audio::SUPPORTED_SAMPLE_RATES {
            if config.try_with_sample_rate(rate).is_some() {
                return Some(rate);
            }
        }
        None
    }
}

#[cfg(feature = "audio")]
impl From<StreamConfig> for SupportedStreamConfigRange {
    fn from(val: StreamConfig) -> Self {
        SupportedStreamConfigRange::new(
            val.channels as ChannelCount,
            val.sample_rate,
            val.sample_rate,
            rodio::cpal::SupportedBufferSize::Range {
                min: val.buffer_size_min,
                max: val.buffer_size_max,
            },
            match val.sample_format.as_str() {
                "f32" => SampleFormat::F32,
                "i32" => SampleFormat::I32,
                "i16" => SampleFormat::I16,
                _ => SampleFormat::F32,
            },
        )
    }
}

#[cfg(feature = "audio")]
impl From<StreamConfig> for rodio::cpal::StreamConfig {
    fn from(val: StreamConfig) -> Self {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        let buffer_size = rodio::cpal::BufferSize::Default;

        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        let buffer_size = rodio::cpal::BufferSize::Fixed(crate::consts::audio::BUFFER_SIZE);

        rodio::cpal::StreamConfig {
            channels: val.channels as ChannelCount,
            sample_rate: val.sample_rate,
            buffer_size,
        }
    }
}

#[cfg(feature = "audio")]
impl From<StreamConfig> for rodio::cpal::SupportedStreamConfig {
    fn from(val: StreamConfig) -> Self {
        rodio::cpal::SupportedStreamConfig::new(
            val.channels as ChannelCount,
            val.sample_rate,
            rodio::cpal::SupportedBufferSize::Range {
                min: val.buffer_size_min,
                max: val.buffer_size_max,
            },
            match val.sample_format.as_str() {
                "f32" => SampleFormat::F32,
                "i32" => SampleFormat::I32,
                "i16" => SampleFormat::I16,
                _ => SampleFormat::F32,
            },
        )
    }
}
