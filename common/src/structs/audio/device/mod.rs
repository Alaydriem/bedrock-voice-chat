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

impl AudioDevice {
    /// Collapses entries nothing downstream can tell apart, keeping the highest rate.
    ///
    /// The ASIO enumeration emits one entry per supported config and builds the name from
    /// the channel count alone, so a driver offering 48 kHz and 44.1 kHz on one channel
    /// yields two entries with the same name. The capture config is re-queried by device
    /// id when the stream starts, so the extras resolve to the same hardware and the same
    /// rate: they are choices the user cannot actually make, and a picker keyed on the
    /// name cannot render them at all.
    ///
    /// Identity is the name as well as the id. One ASIO device carries each of its
    /// channels under a single id, so collapsing on the id alone would offer channel 1
    /// and silently discard the rest.
    pub fn deduplicate(devices: Vec<Self>) -> Vec<Self> {
        let mut kept = Vec::<Self>::with_capacity(devices.len());

        for device in devices {
            match kept
                .iter()
                .position(|existing| existing.is_same_endpoint(&device))
            {
                // Assigned in place rather than pushed: a device must not move up the
                // list because a duplicate of something above it was dropped.
                Some(index) if device.best_sample_rate() > kept[index].best_sample_rate() => {
                    kept[index] = device;
                }
                Some(_) => {}
                None => kept.push(device),
            }
        }

        kept
    }

    /// The same endpoint, under the same host, offered under the same name.
    fn is_same_endpoint(&self, other: &Self) -> bool {
        self.io == other.io
            && self.host == other.host
            && self.id == other.id
            && self.display_name == other.display_name
    }

    /// The best rate this entry offers, which is what ranks it against a duplicate.
    fn best_sample_rate(&self) -> u32 {
        self.stream_configs
            .iter()
            .map(|config| config.sample_rate)
            .max()
            .unwrap_or(0)
    }
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
