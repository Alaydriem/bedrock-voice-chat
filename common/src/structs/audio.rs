use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

#[cfg(feature = "audio")]
use anyhow::anyhow;
#[cfg(feature = "audio")]
use rodio::cpal::{self, ChannelCount, HostId, SampleFormat, SampleRate, SupportedStreamConfigRange};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct NoiseGateSettings {
    pub open_threshold: f32,
    pub close_threshold: f32,
    pub release_rate: f32,
    pub attack_rate: f32,
    pub hold_time: f32,
}

impl Default for NoiseGateSettings {
    fn default() -> Self {
        Self {
            open_threshold: -36.0,
            close_threshold: -56.0,
            release_rate: 150.0,
            attack_rate: 5.0,
            hold_time: 150.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlayerGainSettings {
    pub gain: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlayerGainStore(pub HashMap<String, PlayerGainSettings>);

impl Default for PlayerGainStore {
    fn default() -> Self {
        Self(std::collections::HashMap::new())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum StreamEvent {
    Mute,
    Record
}

/// Audio output format selection for rendering recordings
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AudioFormat {
    /// Broadcast WAV with BEXT metadata (uncompressed PCM)
    Bwav,
    /// MP4/M4A with Opus audio (compressed, lossless passthrough)
    Mp4Opus,
}

impl AudioFormat {
    /// Returns the file extension for this format (without dot)
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Bwav => "wav",
            AudioFormat::Mp4Opus => "m4a",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AudioDeviceType {
    InputDevice,
    OutputDevice,
}

impl AudioDeviceType {
    pub fn to_string(&self) -> String {
        match self {
            AudioDeviceType::InputDevice => "input_audio_device".to_string(),
            AudioDeviceType::OutputDevice => "output_audio_device".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct StreamConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub sample_format: String,
    pub buffer_size_min: u32,
    pub buffer_size_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AudioDevice {
    pub io: AudioDeviceType,
    pub name: String,
    pub host: AudioDeviceHost,
    pub stream_configs: Vec<StreamConfig>,
    pub display_name: String,
}

// --- Feature-gated cpal conversion impls ---

#[cfg(feature = "audio")]
pub const BUFFER_SIZE: u32 = 960;

/// Supported sample rates in order of preference (highest first)
#[cfg(feature = "audio")]
pub const SUPPORTED_SAMPLE_RATES: [u32; 2] = [48000, 44100];

/// Returns the best supported sample rate for a config, preferring higher rates
#[cfg(feature = "audio")]
pub fn get_best_sample_rate(config: &SupportedStreamConfigRange) -> Option<u32> {
    for rate in SUPPORTED_SAMPLE_RATES {
        if config.try_with_sample_rate(SampleRate(rate)).is_some() {
            return Some(rate);
        }
    }
    None
}

#[cfg(feature = "audio")]
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

#[cfg(feature = "audio")]
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

/// Maps the Stream Config to a rodio::cpal::SupportedStreamConfigRange
#[cfg(feature = "audio")]
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
#[cfg(feature = "audio")]
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
#[cfg(feature = "audio")]
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

#[cfg(feature = "audio")]
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