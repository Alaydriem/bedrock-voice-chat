use anyhow::anyhow;
use rodio::{
    cpal::{
        self, traits::HostTrait, ChannelCount, HostId, SampleFormat, SampleRate,
        SupportedStreamConfigRange,
    },
    DeviceTrait,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const BUFFER_SIZE: u32 = 960;

/// Supported sample rates in order of preference (highest first)
pub const SUPPORTED_SAMPLE_RATES: [u32; 2] = [48000, 44100];

/// Returns the best supported sample rate for a config, preferring higher rates
pub fn get_best_sample_rate(config: &SupportedStreamConfigRange) -> Option<u32> {
    for rate in SUPPORTED_SAMPLE_RATES {
        if config.try_with_sample_rate(SampleRate(rate)).is_some() {
            return Some(rate);
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../src/js/bindings/")]
pub enum AudioDeviceType {
    InputDevice,
    OutputDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../src/js/bindings/")]
pub enum AudioDeviceHost {
    #[cfg(target_os = "windows")]
    Asio,
    #[cfg(target_os = "windows")]
    Wasapi,
    #[cfg(target_os = "android")]
    AAudio,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    CoreAudio,
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    Alsa,
}

impl TryFrom<rodio::cpal::HostId> for AudioDeviceHost {
    type Error = ();

    fn try_from(value: rodio::cpal::HostId) -> Result<Self, Self::Error> {
        #[allow(unreachable_patterns)]
        match value {
            #[cfg(target_os = "windows")]
            HostId::Asio => Ok(AudioDeviceHost::Asio),
            #[cfg(target_os = "windows")]
            HostId::Wasapi => Ok(AudioDeviceHost::Wasapi),
            #[cfg(target_os = "android")]
            HostId::AAudio => Ok(AudioDeviceHost::AAudio),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            HostId::CoreAudio => Ok(AudioDeviceHost::CoreAudio),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd"
            ))]
            HostId::Alsa => Ok(AudioDeviceHost::Alsa),
            _ => Err(()),
        }
    }
}

impl Into<rodio::cpal::HostId> for AudioDeviceHost {
    fn into(self) -> rodio::cpal::HostId {
        let host: rodio::cpal::HostId;
        #[cfg(target_os = "windows")]
        {
            host = match self {
                AudioDeviceHost::Asio => HostId::Asio,
                AudioDeviceHost::Wasapi => HostId::Wasapi,
            };
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ))]
        {
            host = match self {
                AudioDeviceHost::Alsa => HostId::Alsa,
            };
        }
        #[cfg(target_os = "android")]
        {
            host = match self {
                AudioDeviceHost::AAudio => HostId::AAudio,
            };
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            host = match self {
                AudioDeviceHost::CoreAudio => HostId::CoreAudio,
            };
        }

        host
    }
}

impl AudioDeviceType {
    pub fn to_string(&self) -> String {
        match self {
            AudioDeviceType::InputDevice => "input_audio_device".to_string(),
            AudioDeviceType::OutputDevice => "output_audio_device".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../src/js/bindings/")]
pub struct AudioDevice {
    pub io: AudioDeviceType,
    pub name: String,
    pub host: AudioDeviceHost,
    pub stream_configs: Vec<StreamConfig>,
    pub display_name: String,
}

impl AudioDevice {
    pub fn new(
        io: AudioDeviceType,
        name: String,
        host: AudioDeviceHost,
        supported_stream_configs: Vec<SupportedStreamConfigRange>,
        display_name: String,
    ) -> Self {
        Self {
            io,
            name,
            host,
            stream_configs: AudioDevice::to_stream_config(supported_stream_configs),
            display_name,
        }
    }

    /// Converts cpal SupportedStreamConfigs into a Vec of Samples and Bitrates we are
    /// willing to support: 48kHz or 44.1kHz (preferring 48kHz), and f32, i16, and i32 samples
    pub fn to_stream_config(
        supported_stream_configs: Vec<SupportedStreamConfigRange>,
    ) -> Vec<StreamConfig> {
        let mut stream_configs = Vec::<StreamConfig>::new();

        for c in supported_stream_configs {
            // Check if config supports one of our required sample rates and has a valid format
            let best_sample_rate = get_best_sample_rate(&c);
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
                        SampleFormat::F32 => "f32",
                        SampleFormat::I16 => "i16",
                        SampleFormat::I32 => "i32",
                        _ => "f32",
                    }
                    .to_string(),
                    buffer_size_min,
                    buffer_size_max,
                });
            }
        }

        // Sort by sample rate descending so 48kHz configs come first
        stream_configs.sort_by(|a, b| b.sample_rate.cmp(&a.sample_rate));

        stream_configs
    }

    // Returns the first valid stream config for the device
    pub fn get_stream_config(&self) -> Result<cpal::SupportedStreamConfig, anyhow::Error> {
        match self.stream_configs.len() {
            0 => Err(anyhow!(
                "{} {} does not have any supported stream configs.",
                self.io.to_string(),
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

/// Maps the AudioDevice back to a raw cpal device
#[allow(unreachable_patterns)]
impl Into<Option<rodio::cpal::Device>> for AudioDevice {
    fn into(self) -> Option<rodio::cpal::Device> {
        let host: rodio::cpal::Host;

        #[cfg(target_os = "windows")]
        {
            host = match self.host {
                AudioDeviceHost::Asio => rodio::cpal::host_from_id(HostId::Asio).unwrap(),
                AudioDeviceHost::Wasapi => rodio::cpal::host_from_id(HostId::Wasapi).unwrap(),
                _ => return None,
            };
        }

        #[cfg(target_os = "android")]
        {
            host = match self.host {
                AudioDeviceHost::AAudio => rodio::cpal::host_from_id(HostId::AAudio).unwrap(),
                _ => return None,
            };
        }

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            host = match self.host {
                AudioDeviceHost::CoreAudio => rodio::cpal::host_from_id(HostId::CoreAudio).unwrap(),
                _ => return None,
            };
        }
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ))]
        {
            host = match self.host {
                AudioDeviceHost::Alsa => rodio::cpal::host_from_id(HostId::Alsa).unwrap(),
                _ => return None,
            };
        }

        match self.io {
            AudioDeviceType::InputDevice => {
                if self.name == "default" {
                    return host.default_input_device();
                }

                match host.input_devices() {
                    Ok(mut devices) => {
                        devices.find(|x| x.name().map(|y| y == self.name).unwrap_or(false))
                    }
                    Err(_) => None,
                }
            }
            AudioDeviceType::OutputDevice => {
                if self.name == "default" {
                    return host.default_output_device();
                }

                match host.output_devices() {
                    Ok(mut devices) => {
                        devices.find(|x| x.name().map(|y| y == self.name).unwrap_or(false))
                    }
                    Err(_) => None,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../src/js/bindings/")]
pub struct StreamConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub sample_format: String,
    pub buffer_size_min: u32,
    pub buffer_size_max: u32,
}

/// Maps the Stream Config to a rodio::cpal::SupportedStreamConfigRange
impl Into<SupportedStreamConfigRange> for StreamConfig {
    fn into(self) -> SupportedStreamConfigRange {
        SupportedStreamConfigRange::new(
            self.channels as ChannelCount,
            SampleRate(self.sample_rate),
            SampleRate(self.sample_rate),
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
impl Into<rodio::cpal::StreamConfig> for StreamConfig {
    fn into(self) -> rodio::cpal::StreamConfig {
        rodio::cpal::StreamConfig {
            channels: self.channels as ChannelCount,
            sample_rate: SampleRate(self.sample_rate),
            buffer_size: rodio::cpal::BufferSize::Fixed(BUFFER_SIZE),
        }
    }
}

/// Maps the Stream Config to a rodio::cpal::SupportedStreamConfig
impl Into<rodio::cpal::SupportedStreamConfig> for StreamConfig {
    fn into(self) -> rodio::cpal::SupportedStreamConfig {
        rodio::cpal::SupportedStreamConfig::new(
            self.channels as ChannelCount,
            SampleRate(self.sample_rate),
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
