use std::{ collections::HashMap, sync::Arc };

use anyhow::anyhow;
use common::structs::{ audio::{ AudioDevice, AudioDeviceType }, config::StreamType };
use rodio::cpal::{ Device, traits::{ DeviceTrait, HostTrait } };
use moka::sync::Cache;
use rand::distributions::{ Alphanumeric, DistString };

use async_once_cell::OnceCell;

pub(crate) mod input;
pub(crate) mod output;

const BUFFER_SIZE: u32 = 960;

/// Individual opus decoded audio packets
pub(crate) struct RawAudioFramePacket {
    pub author: String,
    pub pcm: Vec<f32>,
    pub in_group: Option<bool>,
}

pub(crate) static STREAM_STATE_CACHE: OnceCell<
    Option<Arc<Cache<String, String, std::collections::hash_map::RandomState>>>
> = OnceCell::new();

const INPUT_STREAM: &str = "input_stream";
const OUTPUT_STREAM: &str = "output_stream";

/// Returns true if the network stream is active by measurement of a cache key being present
#[tauri::command(async)]
pub(crate) async fn is_audio_stream_active() -> bool {
    match STREAM_STATE_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) =>
                    match cache.get(INPUT_STREAM) {
                        Some(_) => {
                            return true;
                        }
                        None => {
                            return false;
                        }
                    }
                None => {
                    return false;
                }
            }
        None => {
            return false;
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn stop_stream(st: StreamType) -> bool {
    let cache_key = match st {
        StreamType::InputStream => INPUT_STREAM,
        StreamType::OutputStream => OUTPUT_STREAM,
    };

    match STREAM_STATE_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) => {
                    let jobs: HashMap<String, i8> = HashMap::<String, i8>::new();
                    cache.insert(cache_key.to_string(), serde_json::to_string(&jobs).unwrap());
                    return true;
                }
                None => {
                    return false;
                }
            }
        None => {
            return false;
        }
    }
}

/// Sets up the task cache with the correct values
/// We're storing the current job inside of the cache as a single value
/// When this task launches, we replace the entire cache key with single element containing only this id
/// We're using a hashmap to make a single lookup with a HashMap::<String, id>::new() value
/// Where the String is the self identifier of _this_ thread, and the id is the job running status
/// When this thread launches, we consider all other threads invalid, and burn the entire cache
/// If for some reason we can't access the cache, then this thread self terminates
async fn setup_task_cache(
    cache_key: &str
) -> Result<(String, &Arc<Cache<String, String>>), anyhow::Error> {
    // Self assign an ID for this job
    let id = Alphanumeric.sample_string(&mut rand::thread_rng(), 24);

    match STREAM_STATE_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) => {
                    let mut jobs: HashMap<String, i8> = HashMap::<String, i8>::new();
                    jobs.insert(id.clone(), 1);

                    cache.insert(cache_key.to_string(), serde_json::to_string(&jobs).unwrap());
                    return Ok((id, cache));
                }
                None => {
                    return Err(anyhow!("Cache wasn't found."));
                }
            }
        None => {
            return Err(anyhow!("Cache doesn't exist."));
        }
    }
}

pub(crate) fn get_cpal_hosts() -> Result<Vec<cpal::platform::Host>, anyhow::Error> {
    let mut hosts: Vec<cpal::platform::Host> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        match cpal::host_from_id(cpal::HostId::Wasapi) {
            Ok(host) => hosts.push(host),
            Err(e) => {
                tracing::error!("{}", e.to_string());
                return Err(anyhow!("Could not initialize WASAPI Audio Host."));
            }
        }

        /*
        /// asio-sys isn't returning any devices for the specified interface
        /// For now, ignore this. We can add ASIO support later
        /// @todo!()
        match cpal::host_from_id(cpal::HostId::Asio) {
            Ok(host) => hosts.push(host),
            Err(_) => {
                tracing::warn!(
                    "ASIO host either couldn't be initialized, or isn't available on this system."
                );
            }
        }
        */
    }

    // I guess you could run this on a Mac and be playing on a mobile device ?
    #[cfg(target_os = "macos")]
    {
        match cpal::host_from_id(cpal::HostId::CoreAudio) {
            Ok(host) => hosts.push(host),
            Err(e) => {
                tracing::error!("{}", e.to_string());
                return Err(anyhow!("Could not initialize CoreAudio Audio Host."));
            }
        };
    }

    return Ok(hosts);
}

/// Returns a list of  audio devices (input and outputs) for all drivers available on this system
/// On Windows, ASIO devices may also be returned if an exclusive audio lock can be achieved
#[tauri::command(async)]
pub(crate) async fn get_devices() -> Result<HashMap<String, Vec<AudioDevice>>, bool> {
    let hosts = match get_cpal_hosts() {
        Ok(hosts) => hosts,
        Err(_) => {
            return Err(false);
        }
    };

    let mut devices = HashMap::<String, Vec<AudioDevice>>::new();

    for host in hosts {
        let mut device_map = Vec::<AudioDevice>::new();

        match host.input_devices() {
            Ok(devices) => {
                for device in devices {
                    let name = match device.name() {
                        Ok(name) => name,
                        Err(e) => {
                            tracing::warn!("{}", e.to_string());
                            continue;
                        }
                    };
                    device_map.push(AudioDevice {
                        io: AudioDeviceType::InputDevice,
                        name,
                    });
                }
            }
            Err(e) => {
                tracing::warn!("{}", e.to_string());
            }
        }

        match host.output_devices() {
            Ok(devices) => {
                for device in devices {
                    let name = match device.name() {
                        Ok(name) => name,
                        Err(e) => {
                            tracing::warn!("{}", e.to_string());
                            continue;
                        }
                    };
                    device_map.push(AudioDevice {
                        io: AudioDeviceType::OutputDevice,
                        name,
                    });
                }
            }
            Err(e) => {
                tracing::warn!("{}", e.to_string());
            }
        }

        devices.insert(host.id().name().to_string(), device_map);
    }

    return Ok(devices);
}

/// Returns the device by name, type, and by host preference, if it exists.
async fn get_device(
    device: String,
    st: AudioDeviceType,
    _prefered_host: Option<String>
) -> Result<Device, anyhow::Error> {
    let hosts = match get_cpal_hosts() {
        Ok(hosts) => hosts,
        Err(e) => {
            return Err(anyhow!(e.to_string()));
        }
    };

    let host = hosts.get(0).unwrap();

    let devices_list = match st {
        AudioDeviceType::InputDevice => host.input_devices(),
        AudioDeviceType::OutputDevice => host.output_devices(),
    };

    let mut devices = match devices_list {
        Ok(devices) => devices,
        Err(e) => {
            return Err(anyhow!(e.to_string()));
        }
    };

    let device = match device.as_str() {
        "default" =>
            match st {
                AudioDeviceType::InputDevice => host.default_input_device().unwrap(),
                AudioDeviceType::OutputDevice => host.default_output_device().unwrap(),
            }
        _ =>
            match
                devices.find(|x|
                    x
                        .name()
                        .map(|y| y == device)
                        .unwrap_or(false)
                )
            {
                Some(device) => device,
                None => {
                    return Err(anyhow!("Device {} was not found", device));
                }
            }
    };

    return Ok(device);
}
