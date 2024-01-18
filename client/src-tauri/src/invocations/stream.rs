use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use common::structs::{
    audio::{AudioDevice, AudioDeviceType},
    config::StreamType,
    packet::{AudioFramePacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData},
};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, StreamConfig,
};
use moka::sync::Cache;
use rand::distributions::{Alphanumeric, DistString};

use async_once_cell::OnceCell;
use audio_gate::NoiseGate;
use cpal::SampleRate;
use kanal::Sender;
use rtrb::RingBuffer;
use tauri::State;

const BUFFER_SIZE: u32 = 960;

//pub(crate) type AudioFramePacketConsumer = Arc<Mutex<AsyncReceiver<AudioFramePacket>>>;

//pub(crate) type AudioFramePacketProducer = Arc<Mutex<AsyncSender<AudioFramePacket>>>;

pub(crate) static STREAM_STATE_CACHE: OnceCell<
    Option<Arc<Cache<String, String, std::collections::hash_map::RandomState>>>,
> = OnceCell::new();

const INPUT_STREAM: &str = "input_stream";
const OUTPUT_STREAM: &str = "output_stream";

/// Handles the input audio stream
/// The input audio stream captures audio from a named input device
/// Processes it through AudioGate, other filters, then libopus
/// Then sends it to the QuicNetwork handler to be sent across the network.
#[tauri::command(async)]
pub(crate) async fn input_stream(
    device: String,
    tx: State<'_, Arc<Sender<QuicNetworkPacket>>>,
) -> Result<bool, bool> {
    // Stop existing input streams
    stop_stream(StreamType::InputStream).await;

    let (id, cache) = match setup_task_cache(INPUT_STREAM).await {
        Ok((id, cache)) => (id, cache),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let gamertag = match super::credentials::get_credential("gamertag".into()).await {
        Ok(gt) => gt,
        Err(_) => {
            return Err(false);
        }
    };

    let tx = tx.inner().clone();

    let device = match get_device(device, AudioDeviceType::InputDevice, None).await {
        Ok(device) => device,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    // Input should be a mono channel
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
    };

    let config_c = config.clone();

    println!("{}", device.name().unwrap());

    let latency_frames = (250.0 / 1_000.0) * (config.sample_rate.0 as f32);
    let latency_samples = (latency_frames as usize) * (config.channels as usize);
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(latency_samples * 4);

    let mut gate = NoiseGate::new(-36.0, -54.0, 48000.0, 2, 150.0, 25.0, 150.0);

    let stream = match device.build_input_stream(
        &config_c,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let gated_data = gate.process_frame(&data);
            for &sample in gated_data.as_slice() {
                //for &sample in data {
                producer.push(sample).unwrap_or({});
            }
        },
        move |_| {},
        None, // None=blocking, Some(Duration)=timeout
    ) {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("Failed to build input audio stream {}", e.to_string());
            return Err(false);
        }
    };

    stream.play().unwrap();

    let tx = tx.clone();
    let mut data_stream = Vec::<f32>::new();
    let mut encoder =
        opus::Encoder::new(48000, opus::Channels::Mono, opus::Application::LowDelay).unwrap();

    #[allow(irrefutable_let_patterns)]
    while let sample = consumer.pop() {
        match sample {
            Ok(sample) => {
                let id = id.clone();

                if super::should_self_terminate_sync(&id, &cache.clone(), INPUT_STREAM) {
                    break;
                }

                data_stream.push(sample);

                if data_stream.len() == (BUFFER_SIZE as usize) {
                    let dsc = data_stream.clone();
                    data_stream = Vec::<f32>::new();

                    let s = match encoder.encode_vec_float(&dsc, dsc.len() * 2) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("{}", e.to_string());
                            Vec::<u8>::with_capacity(0)
                        }
                    };

                    if s.len() <= 3 {
                        continue;
                    }

                    let packet = QuicNetworkPacket {
                        client_id: vec![0; 0],
                        author: gamertag.clone(),
                        packet_type: PacketType::AudioFrame,
                        data: QuicNetworkPacketData::AudioFrame(AudioFramePacket {
                            length: s.len(),
                            data: s.clone(),
                            sample_rate: config.sample_rate.0,
                            author: gamertag.clone(),
                            coordinate: None,
                        }),
                    };

                    let tx = tx.clone();
                    _ = tx.send(packet);
                }
            }
            Err(_) => {}
        }
    }

    stream.pause().unwrap();
    drop(stream);

    return Ok(true);
}

#[tauri::command(async)]
pub(crate) async fn output_stream<'r>(
    device: String,
    rx: State<'r, kanal::Receiver<AudioFramePacket>>,
) -> Result<bool, bool> {
    // Stop existing input streams
    stop_stream(StreamType::InputStream).await;

    let (id, cache) = match setup_task_cache(OUTPUT_STREAM).await {
        Ok((id, cache)) => (id, cache),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let rx = rx.inner().clone();

    let device = match get_device(device, AudioDeviceType::OutputDevice, None).await {
        Ok(device) => device,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    // Input should be a mono channel
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
    };
    let config_c = config.clone();

    println!("{}", device.name().unwrap());

    let latency_frames = (250.0 / 1_000.0) * (config.sample_rate.0 as f32);
    let latency_samples = (latency_frames as usize) * (config.channels as usize);
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(latency_samples * 4);

    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    let stream = match device.build_output_stream(
        &config_c,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // react to stream events and read or write stream data here.

            // Attenuate volume based on distance
            // Attenuate based on mute
            // Attenuate based on user volume settings
            // Ambisonic 3D audio

            for sample in data {
                *sample = match consumer.pop() {
                    Ok(s) => s,
                    Err(_) => 0.0,
                };
            }
        },
        move |_| {
            // react to errors here.
        },
        None, // None=blocking, Some(Duration)=timeout
    ) {
        Ok(stream) => stream,
        Err(e) => {
            tracing::info!("Failed to build output audio stream {}", e.to_string());
            return Err(false);
        }
    };

    stream.play().unwrap();

    let rx = rx.clone();
    let mut decoder = opus::Decoder::new(48000, opus::Channels::Mono).unwrap();

    #[allow(irrefutable_let_patterns)]
    while let frame = rx.recv() {
        match frame {
            Ok(frame) => {
                tracing::info!("Received audio frame.");
                let id = id.clone();

                if super::should_self_terminate_sync(&id, &cache.clone(), OUTPUT_STREAM) {
                    break;
                }

                let mut out = vec![0.0; BUFFER_SIZE as usize];
                let out_len = match decoder.decode_float(&frame.data, &mut out, false) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("{}", e.to_string());
                        0
                    }
                };

                out.truncate(out_len);

                if out.len() > 0 {
                    for sample in &out {
                        producer.push(sample.to_owned()).unwrap_or({});
                    }
                }
            }
            Err(_) => {}
        }
    }

    Ok(true)
}

/// Returns true if the network stream is active by measurement of a cache key being present
#[tauri::command(async)]
pub(crate) async fn is_audio_stream_active() -> bool {
    match STREAM_STATE_CACHE.get() {
        Some(cache) => match cache {
            Some(cache) => match cache.get(INPUT_STREAM) {
                Some(_) => {
                    return true;
                }
                None => {
                    return false;
                }
            },
            None => {
                return false;
            }
        },
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
        Some(cache) => match cache {
            Some(cache) => {
                let jobs: HashMap<String, i8> = HashMap::<String, i8>::new();
                cache.insert(cache_key.to_string(), serde_json::to_string(&jobs).unwrap());
                return true;
            }
            None => {
                return false;
            }
        },
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
    cache_key: &str,
) -> Result<(String, &Arc<Cache<String, String>>), anyhow::Error> {
    // Self assign an ID for this job
    let id = Alphanumeric.sample_string(&mut rand::thread_rng(), 24);

    match STREAM_STATE_CACHE.get() {
        Some(cache) => match cache {
            Some(cache) => {
                let mut jobs: HashMap<String, i8> = HashMap::<String, i8>::new();
                jobs.insert(id.clone(), 1);

                cache.insert(cache_key.to_string(), serde_json::to_string(&jobs).unwrap());
                return Ok((id, cache));
            }
            None => {
                return Err(anyhow!("Cache wasn't found."));
            }
        },
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
    _prefered_host: Option<String>,
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
        "default" => match st {
            AudioDeviceType::InputDevice => host.default_input_device().unwrap(),
            AudioDeviceType::OutputDevice => host.default_output_device().unwrap(),
        },
        _ => match devices.find(|x| x.name().map(|y| y == device).unwrap_or(false)) {
            Some(device) => device,
            None => {
                return Err(anyhow!("Device {} was not found", device));
            }
        },
    };

    return Ok(device);
}
