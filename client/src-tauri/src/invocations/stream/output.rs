use std::{
    collections::{ hash_map::RandomState, HashMap },
    sync::{ atomic::{ AtomicBool, Ordering }, Arc },
    time::Duration,
};

use async_mutex::Mutex;
use common::{
    structs::{
        audio::AudioDeviceType,
        config::StreamType,
        packet::{ AudioFramePacket, QuicNetworkPacketCollection },
    },
    Player,
};
use moka::sync::Cache;
use rodio::{
    buffer::SamplesBuffer,
    cpal::{ traits::DeviceTrait, BufferSize, SampleFormat, SampleRate, SupportedBufferSize },
    OutputStream,
    Sink,
    SpatialSink,
};
use anyhow::anyhow;
use flume::Receiver;
use tauri::State;
use async_once_cell::OnceCell;

use crate::invocations::credentials::get_credential;

use super::RawAudioFramePacket;

use std::sync::mpsc;

use tracing::info;
pub(crate) static PLAYER_POSITION_CACHE: OnceCell<
    Option<Arc<Cache<String, Player, RandomState>>>
> = OnceCell::new();

#[tauri::command(async)]
pub(crate) async fn output_stream<'r>(
    device: String,
    rx: State<'r, Arc<Receiver<QuicNetworkPacketCollection>>>
) -> Result<bool, bool> {
    PLAYER_POSITION_CACHE.get_or_init(async {
        return Some(
            Arc::new(
                moka::sync::Cache
                    ::builder()
                    .time_to_live(Duration::from_secs(300))
                    .max_capacity(256)
                    .build()
            )
        );
    }).await;

    let this_player = match get_credential("gamertag") {
        Ok(s) => s,
        Err(_) => {
            tracing::error!("Keychain missing player information, cannot continue.");
            return Err(false);
        }
    };

    // Stop existing input streams
    super::stop_stream(StreamType::OutputStream).await;

    let (mpsc_tx, mpsc_rx) = mpsc::channel();

    let (id, cache) = match super::setup_task_cache(super::OUTPUT_STREAM).await {
        Ok((id, cache)) => (id, cache),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let rx = rx.inner().clone();

    let device = match super::get_device(device, AudioDeviceType::OutputDevice, None).await {
        Ok(device) => device,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let config: cpal::StreamConfig = match device.default_output_config() {
        Ok(config) => {
            let mut config: cpal::StreamConfig = config.into();
            config.buffer_size = BufferSize::Fixed(super::BUFFER_SIZE);
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

    let (producer, consumer) = flume::bounded::<RawAudioFramePacket>(latency_samples * 4);

    tokio::spawn(async move {
        let device: rodio::cpal::Device = device as rodio::cpal::Device;

        let config = rodio::cpal::SupportedStreamConfig::new(
            config_c.channels as u16,
            SampleRate(config.sample_rate.0.into()),
            SupportedBufferSize::Range { min: super::BUFFER_SIZE, max: super::BUFFER_SIZE },
            SampleFormat::F32
        );

        let (_stream, handle) = match OutputStream::try_from_device_config(&device, config) {
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

        let player_cache = match get_player_position_cache() {
            Ok(cache) => cache,
            Err(_) => {
                return;
            }
        };

        let mut spatial_sinks = HashMap::<String, SpatialSink>::new();
        let mut sinks = HashMap::<String, Sink>::new();

        #[allow(irrefutable_let_patterns)]
        while let frame = consumer.recv() {
            let shutdown = shutdown.clone();
            let mut shutdown = shutdown.lock().unwrap();

            if shutdown.get_mut().to_owned() {
                break;
            }

            match frame {
                Ok(frame) => {
                    let author = frame.author;
                    let should_3d_audio = match frame.in_group {
                        Some(in_group) => !in_group,
                        None => true,
                    };
                    let sink = match sinks.get(&author.clone()) {
                        Some(sink) => sink,
                        None => {
                            let sink = Sink::try_new(&handle).unwrap();
                            _ = sinks.insert(author.clone(), sink);

                            sinks.get(&author.clone()).unwrap()
                        }
                    };

                    let spatial_sink = match spatial_sinks.get(&author.clone()) {
                        Some(sink) => sink,
                        None => {
                            let sink = SpatialSink::try_new(
                                &handle,
                                [0.0, 0.0, 0.0],
                                [0.0, 0.0, 0.0],
                                [0.0, 0.0, 0.0]
                            ).unwrap();
                            _ = spatial_sinks.insert(author.clone(), sink);

                            spatial_sinks.get(&author.clone()).unwrap()
                        }
                    };

                    let mut pcm = SamplesBuffer::new(
                        config_c.channels,
                        config_c.sample_rate.0.into(),
                        frame.pcm.clone()
                    );

                    let speaker = player_cache.get(&author);
                    let listener = player_cache.get(&this_player.to_string());

                    // 3d spatial audio attenuation
                    match should_3d_audio {
                        true => {
                            // Volume slider attenuation
                            // !todo();

                            // If we have coordinates for both the speaker, and the listener, then we can do 3D audio translation
                            if speaker.is_some() && listener.is_some() {
                                // Directional Audio
                                let speaker = speaker.unwrap();
                                let listener = listener.unwrap();

                                let s = speaker.coordinates;
                                let l = listener.coordinates;
                                spatial_sink.set_emitter_position([s.x, s.y, s.z]);
                                spatial_sink.set_left_ear_position([l.x + 0.0001, l.y, l.z]);
                                spatial_sink.set_right_ear_position([l.x, l.y, l.z]);

                                let distance = (
                                    (s.x - l.x).powf(2.0) +
                                    (s.y - l.y).powf(2.0) +
                                    (s.z - l.z).powf(2.0)
                                ).sqrt();

                                // Attenuate volume based on distance
                                match speaker.deafen {
                                    true =>
                                        match distance <= 3.0 {
                                            // If the player is sneaking, then they are only audible to the listener within 3 blocks of range
                                            true => spatial_sink.append(pcm),
                                            false => {}
                                        }
                                    false => {
                                        // Otherwise, start volume dropoff at 25 blocks
                                        if distance > 24.0 {
                                            let diff = 55.0 - distance;
                                            spatial_sink.set_volume(diff / 20.0);
                                        }
                                        spatial_sink.append(pcm);
                                    }
                                }
                            }
                        }
                        false => {
                            // Volume slider attenuation @todo()!
                            sink.append(pcm);
                        }
                    }
                }
                Err(_) => {}
            }
        }
        tracing::info!("Output stream ended.");
    });

    let rx = rx.clone();
    tokio::spawn(async move {
        let sample_rate: u32 = config.sample_rate.0.into();

        let player_cache = match get_player_position_cache() {
            Ok(cache) => cache,
            Err(_) => {
                return;
            }
        };

        let mut decoders = HashMap::<Vec<u8>, Arc<Mutex<opus::Decoder>>>::new();
        #[allow(irrefutable_let_patterns)]
        while let packet = rx.recv_async().await {
            match packet {
                Ok(packet) => {
                    for player in packet.positions.players {
                        let p = player.clone();
                        player_cache.insert(player.name.clone(), player);
                    }

                    for frame in packet.frames {
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

                        if
                            super::super::should_self_terminate_sync(
                                &id,
                                &cache.clone(),
                                super::OUTPUT_STREAM
                            )
                        {
                            _ = mpsc_tx.send("terminate");
                            break;
                        }

                        let data: Result<AudioFramePacket, ()> = frame.data.to_owned().try_into();
                        match data {
                            Ok(data) => {
                                let mut out = vec![0.0; super::BUFFER_SIZE as usize];
                                let out_len = match
                                    decoder.decode_float(&data.data, &mut out, false)
                                {
                                    Ok(s) => s,
                                    Err(e) => {
                                        tracing::error!("{}", e.to_string());
                                        0
                                    }
                                };

                                out.truncate(out_len);

                                if out.len() > 0 {
                                    _ = producer.send(RawAudioFramePacket {
                                        author: data.author,
                                        pcm: out,
                                        in_group: frame.in_group,
                                    });
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
                Err(_) => {}
            }
        }
    });
    Ok(true)
}

fn get_player_position_cache() -> Result<Arc<Cache<String, Player>>, anyhow::Error> {
    match PLAYER_POSITION_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) => Ok(cache.clone()),
                None => Err(anyhow!("Cache not found.")),
            }

        None => Err(anyhow!("Cache not found")),
    }
}
