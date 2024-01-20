use std::{ collections::HashMap, sync::{ atomic::{ AtomicBool, Ordering }, Arc }, time::Duration };

use anyhow::anyhow;
use async_mutex::Mutex;
use common::structs::{
    audio::{ AudioDevice, AudioDeviceType },
    config::StreamType,
    packet::{ AudioFramePacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData },
};
use rodio::{
    buffer::SamplesBuffer,
    cpal::{
        traits::{ DeviceTrait, HostTrait, StreamTrait },
        BufferSize,
        Device,
        SupportedBufferSize,
        SampleRate,
        SampleFormat,
    },
    source::SineWave,
    OutputStream,
    OutputStreamHandle,
    Sink,
    Source,
};
use moka::sync::Cache;
use opus::Bitrate;
use rand::distributions::{ Alphanumeric, DistString };

use async_once_cell::OnceCell;
use audio_gate::NoiseGate;
use kanal::Sender;
use rtrb::RingBuffer;
use tauri::State;

use std::sync::mpsc;
const BUFFER_SIZE: u32 = 960;

/// Container that stores the client_id and the underlying AudioFrame so we can map decoders to the correct stream
pub(crate) struct AudioFramePacketContainer {
    pub client_id: Vec<u8>,
    pub frame: AudioFramePacket,
}

/// Container that stores the client ID and the raw PCM data
pub(crate) struct RawAudioFramePacket {
    pub client_id: Vec<u8>,
    pub pcm: Vec<f32>,
}

//pub(crate) type AudioFramePacketConsumer = Arc<Mutex<AsyncReceiver<AudioFramePacket>>>;

//pub(crate) type AudioFramePacketProducer = Arc<Mutex<AsyncSender<AudioFramePacket>>>;

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
    device: String,
    tx: State<'_, Arc<Sender<QuicNetworkPacket>>>
) -> Result<bool, bool> {
    let tx = tx.inner().clone();

    // Stop existing input streams
    stop_stream(StreamType::InputStream).await;

    // We're using a mpsc channel to transfer signals from the thread that passes message to the network stream
    // and the underlying stream itself
    let (mpsc_tx, mpsc_rx) = mpsc::channel();

    // A local Moka cache stores a self-assigned ID we store in this thread
    // If the thread needs to be canceled, we simply remove it and the loop checks if it's present or not.
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

    // The audio device we want to use.
    let device = match get_device(device, AudioDeviceType::InputDevice, None).await {
        Ok(device) => device,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let config: cpal::StreamConfig = match device.default_input_config() {
        Ok(config) => {
            let mut config: cpal::StreamConfig = config.into();
            config.buffer_size = BufferSize::Fixed(BUFFER_SIZE);
            config
        }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let config_c = config.clone();

    tracing::info!("Listening with: {}", device.name().unwrap());

    // This is our main ringbuffer that transfer data from our audio input thread to our network packeter.
    let latency_frames = (250.0 / 1_000.0) * (config.sample_rate.0 as f32);
    let latency_samples = (latency_frames as usize) * (config.channels as usize);
    let (mut producer, consumer) = RingBuffer::<Vec<f32>>::new(latency_samples * 4);

    // Our main listener thread on the CPAL device
    // This will run indefinitely until it receives a mscp signal from the writer.
    tokio::spawn(async move {
        let mut gate = NoiseGate::new(
            -36.0,
            -56.0,
            config.sample_rate.0 as f32,
            config.channels.into(),
            150.0,
            5.0,
            150.0
        );

        let stream = match
            device.build_input_stream(
                &config_c,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Gate
                    let gated_data = gate.process_frame(&data);
                    // Suppression
                    // Makeup Gain
                    // Compresor
                    // Limiter

                    match producer.push(gated_data) {
                        Ok(_) => {}
                        Err(_) => {
                            tracing::error!("Failed to push packet into buffer.");
                        }
                    }
                },
                move |_| {},
                None // None=blocking, Some(Duration)=timeout
            )
        {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to build input audio stream {}", e.to_string());
                return;
            }
        };

        stream.play().unwrap();

        loop {
            // This should only fire once, recv() should hold the thread open
            let message: &'static str = mpsc_rx.recv().unwrap();
            if message.eq("terminate") {
                stream.pause().unwrap();
                break;
            }
        }

        drop(stream);
    });

    // This is our processing thread for our audio frames
    // Each frame is packaged into an opus frame, then sent to the network.rs to be submitted to the server
    let tx = tx.clone();
    let consumer = Arc::new(Mutex::new(consumer));
    tokio::spawn(async move {
        let sample_rate: u32 = config.sample_rate.0.into();
        let id = id.clone();
        let mut data_stream = Vec::<f32>::new();
        let mut encoder = opus::Encoder
            ::new(sample_rate, opus::Channels::Mono, opus::Application::Voip)
            .unwrap();

        _ = encoder.set_bitrate(Bitrate::Bits(32_000));
        tracing::info!("Opus Encoder Bitrate: {:?}", encoder.get_bitrate().ok());

        let consumer = consumer.clone();
        let mut consumer = consumer.lock_arc().await;
        loop {
            let sample = consumer.pop();
            match sample {
                Ok(mut sample) => {
                    let id = id.clone();

                    if super::should_self_terminate_sync(&id, &cache.clone(), INPUT_STREAM) {
                        mpsc_tx.send("terminate").unwrap();
                        break;
                    }

                    data_stream.append(&mut sample);

                    // This should practically only ever fire once
                    // So this code is largely redundant.
                    // @todo!() measure if this is even necessary
                    if data_stream.len() >= (BUFFER_SIZE as usize) {
                        let sample_to_process: Vec<f32> = data_stream
                            .get(0..960)
                            .unwrap()
                            .to_vec();

                        let mut remaining_data = data_stream
                            .get(960..data_stream.len())
                            .unwrap()
                            .to_vec()
                            .into_boxed_slice()
                            .to_vec();
                        data_stream = vec![0.0; 0];
                        data_stream.append(&mut remaining_data);
                        data_stream.shrink_to(data_stream.len());
                        data_stream.truncate(data_stream.len());

                        let s = match
                            encoder.encode_vec_float(
                                &sample_to_process,
                                sample_to_process.len() * 4
                            )
                        {
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
                Err(_e) => {}
            }
        }
    });

    // Calling this from the client will result in an immediate return, but the threads will remain active.
    return Ok(true);
}

#[tauri::command(async)]
pub(crate) async fn output_stream<'r>(
    device: String,
    rx: State<'r, Arc<kanal::Receiver<AudioFramePacketContainer>>>
) -> Result<bool, bool> {
    // Stop existing input streams
    stop_stream(StreamType::InputStream).await;

    let (mpsc_tx, mpsc_rx) = mpsc::channel();

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

    let config: cpal::StreamConfig = match device.default_output_config() {
        Ok(config) => {
            let mut config: cpal::StreamConfig = config.into();
            config.buffer_size = BufferSize::Fixed(BUFFER_SIZE);
            config
        }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let config_c = config.clone();

    tracing::info!("Outputting to: {}", device.name().unwrap());

    let latency_frames = (250.0 / 1_000.0) * (config.sample_rate.0 as f32);
    let latency_samples = (latency_frames as usize) * (config.channels as usize);
    let (producer, mut consumer) = RingBuffer::<RawAudioFramePacket>::new(latency_samples * 4);

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;

        let device: rodio::cpal::Device = device as rodio::cpal::Device;

        let config = rodio::cpal::SupportedStreamConfig::new(
            config_c.channels as u16,
            SampleRate(config.sample_rate.0.into()),
            SupportedBufferSize::Range { min: BUFFER_SIZE, max: BUFFER_SIZE },
            SampleFormat::F32
        );

        let (stream, handle) = match OutputStream::try_from_device_config(&device, config) {
            Ok((s, h)) => (s, h),
            Err(e) => {
                tracing::error!("Failed to construct output audio stream: {}", e.to_string());
                return;
            }
        };

        let shutdown = Arc::new(std::sync::Mutex::new(AtomicBool::new(false)));
        let shutdown_thread = shutdown.clone();
        // This is our shutdown monitor, if we get a request via mspc, when can set our atomic bool variable to true
        // Which will signal the loop to end, and will end the stream
        tokio::spawn(async move {
            let shutdown = shutdown_thread.clone();
            let shutdown = shutdown.lock().unwrap();
            let message: &'static str = mpsc_rx.recv().unwrap();
            if message.eq("terminate") {
                shutdown.store(true, Ordering::Relaxed);
                tracing::info!("Output stream ended");
            }
        });

        let mut sinks = HashMap::<Vec<u8>, Arc<Sink>>::new();
        loop {
            let shutdown = shutdown.clone();
            let mut shutdown = shutdown.lock().unwrap();

            if shutdown.get_mut().to_owned() {
                break;
            }

            match consumer.pop() {
                Ok(frame) => {
                    let client_id = frame.client_id;
                    let sink = match sinks.get(&client_id) {
                        Some(sink) => sink.to_owned(),
                        None => {
                            let sink = Sink::try_new(&handle).unwrap();
                            let sink = Arc::new(sink);
                            sinks.insert(client_id, sink.clone());
                            sink
                        }
                    };

                    let pcm = frame.pcm;
                    let source = SamplesBuffer::new(
                        config_c.channels,
                        config_c.sample_rate.0.into(),
                        pcm
                    );
                    sink.append(source);
                }
                Err(_) => {}
            }
        }
        tracing::info!("Output stream ended.");
    });

    let rx = rx.clone();
    let producer = Arc::new(Mutex::new(producer));
    tokio::spawn(async move {
        let sample_rate: u32 = config.sample_rate.0.into();

        let mut decoders = HashMap::<Vec<u8>, Arc<Mutex<opus::Decoder>>>::new();
        #[allow(irrefutable_let_patterns)]
        while let frame = rx.recv() {
            match frame {
                Ok(frame) => {
                    let client_id = frame.client_id;
                    // Each opus stream has it's own encoder/decoder for state management
                    // We can retain this in a simple hashmap
                    // @todo!(): The HashMap size is unbound on the client.
                    // Until the client restarts this could be a bottlecheck for memory
                    let decoder = match decoders.get(&client_id) {
                        Some(decoder) => decoder.to_owned(),
                        None => {
                            let decoder = opus::Decoder
                                ::new(sample_rate, opus::Channels::Mono)
                                .unwrap();
                            let decoder = Arc::new(Mutex::new(decoder));
                            decoders.insert(client_id.clone(), decoder.clone());
                            decoder
                        }
                    };

                    let mut decoder = decoder.lock_arc().await;
                    let id = id.clone();

                    if super::should_self_terminate_sync(&id, &cache.clone(), OUTPUT_STREAM) {
                        _ = mpsc_tx.send("terminate");
                        break;
                    }

                    let mut out = vec![0.0; BUFFER_SIZE as usize];
                    let out_len = match decoder.decode_float(&frame.frame.data, &mut out, false) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("{}", e.to_string());
                            0
                        }
                    };

                    out.truncate(out_len);

                    if out.len() > 0 {
                        let producer = producer.clone();
                        let mut producer = producer.lock_arc().await;
                        _ = producer.push(RawAudioFramePacket {
                            client_id,
                            pcm: out,
                        });
                        drop(producer);
                    }
                }
                Err(_) => {}
            }
        }
    });

    Ok(true)
}

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
