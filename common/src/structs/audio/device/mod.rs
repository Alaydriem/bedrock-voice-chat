pub mod host;
pub mod r#type;

pub use host::AudioDeviceHost;
pub use r#type::AudioDeviceType;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::stream::config::StreamConfig;

#[cfg(feature = "audio")]
use anyhow::anyhow;
#[cfg(feature = "audio")]
use rodio::cpal::SupportedStreamConfigRange;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AudioDevice {
    pub io: AudioDeviceType,
    pub id: String,
    pub name: String,
    pub host: AudioDeviceHost,
    pub stream_configs: Vec<StreamConfig>,
    pub display_name: String,
}

#[cfg(feature = "audio")]
impl AudioDevice {
    pub fn new(
        io: AudioDeviceType,
        id: String,
        name: String,
        host: AudioDeviceHost,
        supported_stream_configs: Vec<SupportedStreamConfigRange>,
        display_name: String,
    ) -> Self {
        Self {
            io,
            id,
            name,
            host,
            stream_configs: AudioDevice::to_stream_config(supported_stream_configs),
            display_name,
        }
    }

    pub fn to_stream_config(
        supported_stream_configs: Vec<SupportedStreamConfigRange>,
    ) -> Vec<StreamConfig> {
        let mut stream_configs = Vec::<StreamConfig>::new();

        for c in supported_stream_configs {
            let best_sample_rate = super::stream::config::StreamConfig::best_sample_rate(&c);
            let has_valid_format = c.sample_format().eq(&rodio::cpal::SampleFormat::F32)
                || c.sample_format().eq(&rodio::cpal::SampleFormat::I32)
                || c.sample_format().eq(&rodio::cpal::SampleFormat::I16);

            if let (Some(sample_rate), true) = (best_sample_rate, has_valid_format) {
                let (buffer_size_min, buffer_size_max) = match c.buffer_size() {
                    rodio::cpal::SupportedBufferSize::Range { min, max } => {
                        (min.to_owned(), max.to_owned())
                    }
                    _ => (0, 0),
                };

                stream_configs.push(StreamConfig {
                    channels: c.channels(),
                    sample_rate,
                    sample_format: match c.sample_format() {
                        rodio::cpal::SampleFormat::F32 => "f32",
                        rodio::cpal::SampleFormat::I16 => "i16",
                        rodio::cpal::SampleFormat::I32 => "i32",
                        _ => "f32",
                    }
                    .to_string(),
                    buffer_size_min,
                    buffer_size_max,
                });
            }
        }

        stream_configs.sort_by(|a, b| b.sample_rate.cmp(&a.sample_rate));

        stream_configs
    }

    pub fn get_stream_config(&self) -> Result<rodio::cpal::SupportedStreamConfig, anyhow::Error> {
        match self.stream_configs.len() {
            0 => Err(anyhow!(
                "{} {} does not have any supported stream configs.",
                self.io.store_key(),
                self.display_name
            )),
            _ => {
                let configs: Vec<rodio::cpal::SupportedStreamConfig> = self
                    .stream_configs
                    .clone()
                    .iter()
                    .map(|c| Into::<rodio::cpal::SupportedStreamConfig>::into(c.to_owned()))
                    .collect();

                Ok(configs.first().unwrap().clone())
            }
        }
    }
}
