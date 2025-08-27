use std::collections::HashMap;

use crate::audio::types::{AudioDevice, AudioDeviceHost, AudioDeviceType};
use anyhow::anyhow;
use cpal::{
    traits::{DeviceTrait, HostTrait},
    HostId, SupportedStreamConfigRange,
};
use log::{error, warn};

/// Returns a Vec of cpal hosts
/// On Windows, this _should_ be ASIO and WASAPI
/// On MacOS (unsupported), this should be CoreAudio
/// On mobile platforms this should be????
pub(crate) fn get_cpal_hosts() -> Result<Vec<rodio::cpal::platform::Host>, anyhow::Error> {
    let mut hosts: Vec<cpal::platform::Host> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        match cpal::host_from_id(HostId::Wasapi) {
            Ok(host) => hosts.push(host),
            Err(e) => {
                error!("{}", e.to_string());
                return Err(anyhow!("Could not initialize WASAPI Audio Host."));
            }
        }

        match cpal::host_from_id(HostId::Asio) {
            Ok(host) => hosts.push(host),
            Err(_) => {
                warn!(
                    "ASIO host either couldn't be initialized, or isn't available on this system."
                );
            }
        }
    }

    // I guess you could run this on a Mac and be playing on a mobile device ?
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        match cpal::host_from_id(HostId::CoreAudio) {
            Ok(host) => hosts.push(host),
            Err(e) => {
                error!("{}", e.to_string());
                return Err(anyhow!("Could not initialize CoreAudio Audio Host."));
            }
        };
    }

    #[cfg(any(target_os = "android"))]
    {
        match cpal::host_from_id(HostId::Aaudio) {
            Ok(host) => hosts.push(host),
            Err(e) => {
                error!("{}", e.to_string());
                return Err(anyhow!(
                    "Could not initialize AAudio Audio Host for Android."
                ));
            }
        };
    }

    if hosts.len() == 0 {
        return Err(anyhow!("No available CPAL hosts for this platform."));
    }

    return Ok(hosts);
}

pub fn get_devices() -> Result<HashMap<String, Vec<AudioDevice>>, ()> {
    let hosts = match get_cpal_hosts() {
        Ok(hosts) => hosts,
        Err(e) => {
            error!("{}", e.to_string());
            return Err(());
        }
    };

    let mut devices = HashMap::<String, Vec<AudioDevice>>::new();

    for host in hosts {
        let mut device_map = Vec::<AudioDevice>::new();

        match host.input_devices() {
            Ok(devices) => {
                for device in devices {
                    let stream_configs = match device.supported_input_configs() {
                        Ok(cfg) => cfg.map(|s| s).collect(),
                        Err(_) => Vec::new(),
                    };

                    // We need a valid input
                    if stream_configs.len() == 0 {
                        continue;
                    }

                    for audio_device in get_device_name(
                        AudioDeviceType::InputDevice,
                        &host,
                        &device,
                        stream_configs,
                    ) {
                        device_map.push(audio_device);
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Input devices for [{}] are not available. {}",
                    host.id().name(),
                    e.to_string()
                );
            }
        }

        match host.output_devices() {
            Ok(devices) => {
                for device in devices {
                    let stream_configs = match device.supported_output_configs() {
                        Ok(cfg) => cfg.map(|s| s).collect(),
                        Err(_) => Vec::new(),
                    };

                    // We need a valid input
                    if stream_configs.len() == 0 {
                        continue;
                    }

                    for audio_device in get_device_name(
                        AudioDeviceType::OutputDevice,
                        &host,
                        &device,
                        stream_configs,
                    ) {
                        device_map.push(audio_device);
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Otuput devices for [{}] are not available. {}",
                    host.id().name(),
                    e.to_string()
                );
            }
        }

        devices.insert(host.id().name().to_string(), device_map);
    }

    return Ok(devices);
}

fn get_device_name(
    io: AudioDeviceType,
    host: &cpal::Host,
    device: &cpal::Device,
    stream_configs: Vec<SupportedStreamConfigRange>,
) -> Vec<AudioDevice> {
    let device_name = match device.name() {
        Ok(name) => name,
        Err(e) => {
            warn!("{}", e.to_string());
            return vec![];
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
                    device_name.clone(),
                    AudioDeviceHost::try_from(host.id()).unwrap(),
                    vec![supported_config],
                    format!(
                        "{} {} {}",
                        device_name.clone(),
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
            device_name.clone(),
            AudioDeviceHost::try_from(host.id()).unwrap(),
            stream_configs,
            device_name.clone(),
        )],
    }
}
