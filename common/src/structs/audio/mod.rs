pub mod noise_gate_settings;
pub mod player_gain_settings;
pub mod player_gain_store;
pub mod stream_event;
pub mod audio_format;
pub mod audio_device_type;
pub mod mute_event;
pub mod audio_device_host;
pub mod stream_config;
pub mod audio_device;

pub use noise_gate_settings::NoiseGateSettings;
pub use player_gain_settings::PlayerGainSettings;
pub use player_gain_store::PlayerGainStore;
pub use stream_event::StreamEvent;
pub use audio_format::AudioFormat;
pub use audio_device_type::AudioDeviceType;
pub use mute_event::MuteEvent;
pub use audio_device_host::AudioDeviceHost;
pub use stream_config::StreamConfig;
pub use audio_device::AudioDevice;

#[cfg(feature = "audio")]
pub use audio_device::get_best_sample_rate;
