pub use common::structs::audio::{AudioDevice, AudioDeviceHost, AudioDeviceType, StreamConfig};
pub use common::structs::audio::{BUFFER_SIZE, get_best_sample_rate};

use rodio::{
    cpal::{self, traits::HostTrait, HostId},
    DeviceTrait,
};

/// Native sample rate for Opus encoding
pub const OPUS_SAMPLE_RATE: u32 = 48000;

/// Check if resampling is needed (any rate != 48 kHz)
#[allow(dead_code)]
pub fn needs_resampling(device_sample_rate: u32) -> bool {
    device_sample_rate != OPUS_SAMPLE_RATE
}

/// Extension trait for converting AudioDevice to a raw cpal Device.
/// This requires runtime host enumeration so it stays in the client crate.
pub trait AudioDeviceCpal {
    fn to_cpal_device(self) -> Option<cpal::Device>;
}

#[allow(unreachable_patterns)]
impl AudioDeviceCpal for AudioDevice {
    fn to_cpal_device(self) -> Option<cpal::Device> {
        let host: cpal::Host;

        #[cfg(target_os = "windows")]
        {
            host = match self.host {
                AudioDeviceHost::Asio => cpal::host_from_id(HostId::Asio).unwrap(),
                AudioDeviceHost::Wasapi => cpal::host_from_id(HostId::Wasapi).unwrap(),
                _ => return None,
            };
        }

        #[cfg(target_os = "android")]
        {
            host = match self.host {
                AudioDeviceHost::AAudio => cpal::host_from_id(HostId::AAudio).unwrap(),
                _ => return None,
            };
        }

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            host = match self.host {
                AudioDeviceHost::CoreAudio => cpal::host_from_id(HostId::CoreAudio).unwrap(),
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
                AudioDeviceHost::Alsa => cpal::host_from_id(HostId::Alsa).unwrap(),
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
