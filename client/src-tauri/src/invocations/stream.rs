use std::{ thread, time::Duration, collections::HashMap, sync::Arc };

use cpal::{ traits::{ HostTrait, DeviceTrait, StreamTrait }, StreamConfig };
use moka::future::Cache;
use rand::distributions::{ Alphanumeric, DistString };
use anyhow::anyhow;
use common::structs::{
    config::StreamType,
    audio::{ AudioDevice, AudioDeviceType },
    packet::AudioFramePacket,
    packet::PacketType,
    packet::QuicNetworkPacket,
};
use async_mutex::Mutex;
use async_once_cell::OnceCell;
use tauri::State;

use cpal::SampleRate;
use audio_gate::NoiseGate;

use super::network::QuicNetworkPacketProducer;

const BUFFER_SIZE: u32 = 960;

pub(crate) type AudioFramePacketConsumer = Arc<
    Mutex<
        async_ringbuf::AsyncConsumer<
            AudioFramePacket,
            Arc<
                async_ringbuf::AsyncRb<
                    AudioFramePacket,
                    ringbuf::SharedRb<
                        AudioFramePacket,
                        Vec<std::mem::MaybeUninit<AudioFramePacket>>
                    >
                >
            >
        >
    >
>;

pub(crate) type AudioFramePacketProducer = Arc<
    Mutex<
        async_ringbuf::AsyncProducer<
            AudioFramePacket,
            Arc<
                async_ringbuf::AsyncRb<
                    AudioFramePacket,
                    ringbuf::SharedRb<
                        AudioFramePacket,
                        Vec<std::mem::MaybeUninit<AudioFramePacket>>
                    >
                >
            >
        >
    >
>;

pub(crate) static STREAM_STATE_CACHE: OnceCell<
    Option<Arc<Cache<String, String, std::collections::hash_map::RandomState>>>
> = OnceCell::new();

const INPUT_STREAM: &str = "input_stream";
const OUTPUT_STREAM: &str = "output_stream";

/// Handles the input audio stream
/// The input audio stream captures audio from a named input device
/// Processes it through AudioGate, other filters, then libopus
/// Then sends it to the QuicNetwork handler to be sent across the network.
#[tauri::command(async)]
pub(crate) async fn input_stream(
    s: String,
    quic_producer: State<'_, QuicNetworkPacketProducer>
) -> Result<bool, bool> {
    let quic_producer = quic_producer.inner().clone();

    // Create a new job for the thread worker to execute in
    let task: tokio::task::JoinHandle<()> = tokio::task::spawn(async move {
        let gamertag = match super::credentials::get_credential("gamertag".into()).await {
            Ok(gt) => gt,
            Err(_) => {
                return;
            }
        };

        let quic_producer = quic_producer.clone();
        // Create a new task ID and retrieve the cache
        let (id, cache) = match setup_task_cache(INPUT_STREAM).await {
            Ok((id, cache)) => (id, cache),
            Err(e) => {
                tracing::error!("{}", e.to_string());
                // If the cache isn't found, then we have no way to manage state and should self terminate
                return;
            }
        };

        let client_id: Vec<u8> = (0..32).map(|_| { rand::random::<u8>() }).collect();
        let hosts = match get_cpal_hosts() {
            Ok(hosts) => hosts,
            Err(_) => {
                return;
            }
        };

        let host = hosts.get(0).unwrap();

        let mut input_devices = match host.input_devices() {
            Ok(devices) => devices,
            Err(_) => {
                return;
            }
        };

        let device = match
            input_devices.find(|x|
                x
                    .name()
                    .map(|y| y == s)
                    .unwrap_or(false)
            )
        {
            Some(device) => device,
            None => {
                return;
            }
        };

        let config: cpal::StreamConfig = StreamConfig {
            channels: 2,
            sample_rate: SampleRate(48000),
            buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
        };

        let mut encoder = opus::Encoder
            ::new(config.sample_rate.0.into(), opus::Channels::Mono, opus::Application::LowDelay)
            .unwrap();

        // @todo!
        // Noise gate should be configurable on the fly via some cached value
        // So we don't need to instantiate it every time
        // And so we can update it without reloading the input stream
        let mut gate = NoiseGate::new(-36.0, -54.0, 48000.0, 2, 150.0, 25.0, 150.0);

        // We need a ring buffer to collect the audio input, then process it into opus frames
        let latency_frames = (250.0 / 1_000.0) * (config.sample_rate.0 as f32);
        let latency_samples = (latency_frames as usize) * (config.channels as usize);
        let ring = ringbuf::HeapRb::<f32>::new(latency_samples * 2);
        let (mut producer, mut consumer) = ring.split();

        for _ in 0..latency_samples {
            // The ring buffer has twice as much space as necessary to add latency here,
            // so this should never fail
            producer.push(0.0).unwrap();
        }

        // Start the input stream
        match
            device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // @todo!
                    // Update our noise gate from cached values if necessary

                    // Apply our noise gate
                    let gated_data = gate.process_frame(&data);

                    // Apply any other filters to our audio before recording it

                    // @todo!() whisper implementation

                    // Send the samples to the buffer to implement a frame
                    for &sample in gated_data.as_slice() {
                        producer.push(sample).unwrap_or({});
                    }
                },
                move |_err| {
                    return;
                },
                None // None=blocking, Some(Duration)=timeout
            )
        {
            Ok(stream) => stream.play().unwrap(),
            Err(e) => {
                return;
            }
        }

        let mut data_stream = Vec::<f32>::new();
        #[allow(irrefutable_let_patterns)]
        while let sample = consumer.pop() {
            if super::should_self_terminate(&id, cache, INPUT_STREAM).await {
                return;
            }

            // If the audio is muted, don't bother sending it over the network
            // if is_muted() { continue; }

            if sample.is_none() {
                continue;
            }

            data_stream.push(sample.unwrap());

            if data_stream.len() == (BUFFER_SIZE as usize) {
                let dsc = data_stream.clone();
                data_stream = Vec::<f32>::new();

                let s = match encoder.encode_vec_float(&dsc, dsc.len() * 2) {
                    Ok(s) => s,
                    Err(_e) => { Vec::<u8>::with_capacity(0) }
                };

                if s.len() == 0 {
                    continue;
                }

                let packet = QuicNetworkPacket {
                    packet_type: PacketType::AudioFrame,
                    author: gamertag.clone(),
                    client_id: client_id.clone(),
                    data: Box::new(AudioFramePacket {
                        length: s.len(),
                        sample_rate: config.sample_rate.0,
                        data: s.clone(),
                        // @todo!()
                        // origin: Coordinate (get from cache)
                        // author needs to be doubly encoded in the Packet so we can setup separate
                        // opus decoders
                    }),
                };

                let mut quic_producer = quic_producer.lock_arc().await;
                _ = quic_producer.push(packet).await;
            }
        }
    });

    _ = task.await;

    Ok(true)
}

#[tauri::command(async)]
pub(crate) async fn output_stream<'r>(
    s: String,
    consumer: State<'r, AudioFramePacketConsumer>
) -> Result<bool, bool> {
    // Create a new job for the thread worker to execute in
    let task = tokio::task::spawn(async move {
        let (id, cache) = match setup_task_cache(OUTPUT_STREAM).await {
            Ok((id, cache)) => (id, cache),
            Err(e) => {
                tracing::error!("{}", e.to_string());
                return;
            }
        };

        loop {
            //
            if super::should_self_terminate(&id, cache, INPUT_STREAM).await {
                return;
            }

            // Check something external to determine if this thread should be terminated
            thread::sleep(Duration::from_millis(1000));
            println!("Input stream is doing work {}", s);
        }
    });

    _ = task.await;

    Ok(true)
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
                    cache.insert(
                        cache_key.to_string(),
                        serde_json::to_string(&jobs).unwrap()
                    ).await;
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

                    cache.insert(
                        cache_key.to_string(),
                        serde_json::to_string(&jobs).unwrap()
                    ).await;
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
