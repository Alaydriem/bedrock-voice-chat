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

impl StreamConfig {
    /// Highest sample rate first, with one exception: a 64-bit float config ranks below
    /// every other format at any rate. The pipeline is f32 end to end, so an f64 capture
    /// is downcast on arrival and buys nothing. It is accepted only so a device that
    /// offers no other format is still usable.
    pub fn preference_order(mut configs: Vec<Self>) -> Vec<Self> {
        configs.sort_by(|a, b| {
            a.is_last_resort_format()
                .cmp(&b.is_last_resort_format())
                .then(b.sample_rate.cmp(&a.sample_rate))
        });

        configs
    }

    fn is_last_resort_format(&self) -> bool {
        self.sample_format == "f64"
    }
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

    /// The stored name resolves back to the format it was recorded from. A name that
    /// resolves to the wrong format hands the capture callback samples of one width
    /// while the device writes another, which reads as noise rather than as an error.
    fn to_sample_format(&self) -> SampleFormat {
        match self.sample_format.as_str() {
            "f32" => SampleFormat::F32,
            "f64" => SampleFormat::F64,
            "i32" => SampleFormat::I32,
            "i16" => SampleFormat::I16,
            _ => SampleFormat::F32,
        }
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
            val.to_sample_format(),
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
            val.to_sample_format(),
        )
    }
}
