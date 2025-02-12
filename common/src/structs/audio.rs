use rodio::cpal::{ChannelCount, SampleFormat, SampleRate, SupportedStreamConfigRange};
use ts_rs::TS;
use serde::{ Deserialize, Serialize };

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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AudioDevice {
    pub io: AudioDeviceType,
    pub name: String,
    pub host: String,
    pub stream_configs: Vec<StreamConfig>,
    pub display_name: String
}

impl AudioDevice {
    pub fn new(
        io: AudioDeviceType,
        name: String,
        host: String,
        supported_stream_configs: Vec<SupportedStreamConfigRange>,
        display_name: String
    ) -> Self {
        Self {
            io,
            name,
            host,
            stream_configs: AudioDevice::to_stream_config(supported_stream_configs),
            display_name
        }
    }

    /// Converts cpal SupportedStreamConfigs into a Vec of Samples and Bitrates we are
    /// willing to support, 48kHz, and f32, i16, and i32 samples
    pub fn to_stream_config(supported_stream_configs: Vec<SupportedStreamConfigRange>) -> Vec<StreamConfig> {
        let mut stream_configs= Vec::<StreamConfig>::new();

        for c in supported_stream_configs {
            if c.max_sample_rate().eq(&SampleRate(48000))
            && (c.sample_format().eq(&rodio::cpal::SampleFormat::F32) || c.sample_format().eq(&rodio::cpal::SampleFormat::I32)) {
                let (buffer_size_min, buffer_size_max) = match c.buffer_size() {
                    rodio::cpal::SupportedBufferSize::Range { min, max } => (min.to_owned(), max.to_owned()),
                    _ => (0, 0)
                };

                stream_configs.push(StreamConfig {
                    channels: c.channels(),
                    sample_rate: c.max_sample_rate().0,
                    sample_format: match c.sample_format() {
                        SampleFormat::F32 => "f32",
                        SampleFormat::I16 => "i16",
                        SampleFormat::I32 => "i32",
                        _ => "f32"
                    }.to_string(),
                    buffer_size_min,
                    buffer_size_max
                });
            }
        }

        stream_configs
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct StreamConfig {
    pub channels: u16,
    pub sample_rate: u32,
    pub sample_format: String,
    pub buffer_size_min: u32,
    pub buffer_size_max: u32
}

impl Into<SupportedStreamConfigRange> for StreamConfig {
    fn into(self) -> SupportedStreamConfigRange {
        SupportedStreamConfigRange::new(
            self.channels as ChannelCount,
            SampleRate(self.sample_rate),
            SampleRate(self.sample_rate),
            rodio::cpal::SupportedBufferSize::Range { min: self.buffer_size_min, max: self.buffer_size_max },
            match self.sample_format.as_str() {
                "f32" => SampleFormat::F32,
                "i32" => SampleFormat::I32,
                "i16" => SampleFormat::I16,
                _ => SampleFormat::F32 // default case
            }
        )
    }
}