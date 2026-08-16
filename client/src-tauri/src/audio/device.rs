use std::collections::HashMap;

use crate::audio::{AudioDevice, AudioDeviceHost, AudioDeviceType, StreamConfig};
use anyhow::anyhow;
use log::{error, warn};
use rodio::cpal::{
    self, HostId, SupportedStreamConfigRange,
    traits::{DeviceTrait, HostTrait},
};

pub struct AudioDeviceEnumerator;

impl AudioDeviceEnumerator {
    /// Returns a Vec of cpal hosts
    /// On Windows, this _should_ be ASIO and WASAPI
    /// On MacOS (unsupported), this should be CoreAudio
    /// On mobile platforms this should be????
    pub(crate) fn get_cpal_hosts() -> Result<Vec<rodio::cpal::platform::Host>, anyhow::Error> {
        let mut hosts: Vec<cpal::platform::Host> = Vec::new();

        let mut platforms = Vec::<HostId>::new();
        #[cfg(target_os = "windows")]
        {
            platforms.push(HostId::Wasapi);
            platforms.push(HostId::Asio);
        }

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            platforms.push(HostId::CoreAudio);
        }

        #[cfg(any(target_os = "android"))]
        {
            platforms.push(HostId::AAudio);
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ))]
        {
            platforms.push(HostId::Alsa);
        }


        for platform in platforms {
            match cpal::host_from_id(platform) {
                Ok(host) => hosts.push(host),
                Err(e) => {
                    error!("{}", e.to_string());
                    return Err(anyhow!(
                        "Could not initialize {} Audio Host for this platform.",
                        platform.name()
                    ));
                }
            };
        }

        if hosts.len() == 0 {
            return Err(anyhow!("No available CPAL hosts for this platform."));
        }

        return Ok(hosts);
    }

    fn process_devices(
        host: &cpal::Host,
        device_type: AudioDeviceType,
        device_map: &mut Vec<AudioDevice>,
    ) {
        let (devices_result, type_name): (Result<_, cpal::DevicesError>, &str) = match device_type {
            AudioDeviceType::InputDevice => (host.input_devices(), "Input"),
            AudioDeviceType::OutputDevice => (host.output_devices(), "Output"),
        };

        match devices_result {
            Ok(devices) => {
                for device in devices {
                    let stream_configs: Vec<SupportedStreamConfigRange> = match device_type {
                        AudioDeviceType::InputDevice => match device.supported_input_configs() {
                            Ok(cfg) => cfg.map(|s| s).collect(),
                            Err(_) => Vec::new(),
                        },
                        AudioDeviceType::OutputDevice => match device.supported_output_configs() {
                            Ok(cfg) => cfg.map(|s| s).collect(),
                            Err(_) => Vec::new(),
                        },
                    };

                    // We need a valid config
                    if stream_configs.len() == 0 {
                        continue;
                    }

                    // Check if device supports any of our required sample rates (48kHz or 44.1kHz)
                    let supports_required_sample_rates = stream_configs
                        .iter()
                        .any(|config| StreamConfig::best_sample_rate(config).is_some());

                    if !supports_required_sample_rates {
                        continue;
                    }

                    for audio_device in
                        Self::get_device_name(device_type.clone(), &host, &device, stream_configs)
                    {
                        device_map.push(audio_device);
                    }
                }
            }
            Err(e) => {
                warn!(
                    "{} devices for [{}] are not available. {}",
                    type_name,
                    host.id().name(),
                    e.to_string()
                );
            }
        }
    }

    pub fn get_devices() -> Result<HashMap<String, Vec<AudioDevice>>, ()> {
        let hosts = match Self::get_cpal_hosts() {
            Ok(hosts) => hosts,
            Err(e) => {
                error!("{}", e.to_string());
                return Err(());
            }
        };

        let mut devices = HashMap::<String, Vec<AudioDevice>>::new();

        for host in hosts {
            let mut device_map = Vec::<AudioDevice>::new();

            Self::process_devices(&host, AudioDeviceType::InputDevice, &mut device_map);
            Self::process_devices(&host, AudioDeviceType::OutputDevice, &mut device_map);

            devices.insert(
                host.id().name().to_string(),
                AudioDevice::deduplicate(device_map),
            );
        }

        return Ok(devices);
    }

    /// Re-queries CPAL for the current supported configs of a device.
    /// Returns updated stream_configs, or None if device not found or has no valid configs.
    /// This is used to detect when Windows sound settings have changed (e.g., sample rate).
    pub fn refresh_device_config(
        device: &AudioDevice,
    ) -> Option<Vec<crate::audio::StreamConfig>> {
        // Initialize only this device's host. Requiring every platform host to
        // initialize (get_cpal_hosts) lets an unrelated failure — e.g. a broken
        // ASIO driver — block a WASAPI refresh and force a stale stored config.
        let host_id: HostId = device.host.clone().into();
        let host = match cpal::host_from_id(host_id) {
            Ok(host) => host,
            Err(e) => {
                warn!(
                    "Could not initialize {} host to refresh config for {}: {}",
                    host_id.name(),
                    device.display_name,
                    e
                );
                return None;
            }
        };

        let devices_iter = match device.io {
            AudioDeviceType::InputDevice => host.input_devices().ok()?,
            AudioDeviceType::OutputDevice => host.output_devices().ok()?,
        };

        for cpal_device in devices_iter {
            if cpal_device.id().ok().map(|id| id.to_string()) != Some(device.id.clone()) {
                continue;
            }

            // A transient error querying one device must not abort the refresh;
            // keep scanning in case the id matches another enumeration entry.
            let configs: Vec<SupportedStreamConfigRange> = match device.io {
                AudioDeviceType::InputDevice => match cpal_device.supported_input_configs() {
                    Ok(cfg) => cfg.collect(),
                    Err(_) => continue,
                },
                AudioDeviceType::OutputDevice => match cpal_device.supported_output_configs() {
                    Ok(cfg) => cfg.collect(),
                    Err(_) => continue,
                },
            };

            let stream_configs = AudioDevice::to_stream_config(configs);
            if !stream_configs.is_empty() {
                return Some(stream_configs);
            }
        }
        None
    }

    fn get_device_name(
        io: AudioDeviceType,
        host: &cpal::Host,
        device: &cpal::Device,
        stream_configs: Vec<SupportedStreamConfigRange>,
    ) -> Vec<AudioDevice> {
        let device_description = match device.description() {
            Ok(desc) => desc.name().to_string(),
            Err(e) => {
                warn!("Could not get device description: {}", e);
                return vec![];
            }
        };

        let device_id = match device.id() {
            Ok(id) => id.to_string(),
            Err(e) => {
                warn!(
                    "Could not get device ID, falling back to description: {}",
                    e
                );
                device_description.clone()
            }
        };

        #[warn(unreachable_patterns)]
        match host.id() {
            // Each ASIO "channel" is _likely_ a different physical input / output on the device
            // We need to map a "friendly" display name for these since they the ASIO device is one _single_ device, rather than a listing
            //
            #[cfg(target_os = "windows")]
            HostId::Asio => {
                let mut devices = Vec::<AudioDevice>::new();
                // This filters out only the configs we're willing to support for the driver
                // This is super redundant, but get us an iterator we need
                let supported_stream_configs: Vec<SupportedStreamConfigRange> =
                    AudioDevice::to_stream_config(stream_configs)
                        .into_iter()
                        .map(|s| Into::<SupportedStreamConfigRange>::into(s))
                        .collect();
                for supported_config in supported_stream_configs {
                    devices.push(AudioDevice::new(
                        io.clone(),
                        device_id.clone(),
                        device_description.clone(),
                        AudioDeviceHost::try_from(host.id()).unwrap(),
                        vec![supported_config],
                        format!(
                            "{} {} {}",
                            device_description.clone(),
                            match io {
                                AudioDeviceType::InputDevice => "Input",
                                AudioDeviceType::OutputDevice => "Output",
                            },
                            supported_config.channels()
                        ),
                    ))
                }

                devices
            }
            _ => vec![AudioDevice::new(
                io,
                device_id,
                device_description.clone(),
                AudioDeviceHost::try_from(host.id()).unwrap(),
                stream_configs,
                device_description,
            )],
        }
    }
}
