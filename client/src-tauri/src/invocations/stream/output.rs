use std::{ collections::HashMap, sync::{ atomic::{ AtomicBool, Ordering }, Arc }, time::Duration };

use async_mutex::Mutex;
use common::structs::{
    audio::AudioDeviceType,
    config::StreamType,
    packet::{ AudioFramePacket, QuicNetworkPacketCollection },
};
use rodio::{
    buffer::SamplesBuffer,
    cpal::{ traits::DeviceTrait, BufferSize, SupportedBufferSize, SampleRate, SampleFormat },
    source::SineWave,
    OutputStream,
    Sink,
    Source,
};

use flume::Receiver;
use tauri::State;

use super::{ RawAudioFramePacket, RawAudioFramePacketCollection };

use std::sync::mpsc;

#[tauri::command(async)]
pub(crate) async fn output_stream<'r>(
    device: String,
    rx: State<'r, Arc<Receiver<QuicNetworkPacketCollection>>>
) -> Result<bool, bool> {
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

    let (producer, consumer) = flume::bounded::<RawAudioFramePacketCollection>(latency_samples * 4);

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;

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

        let sink = Sink::try_new(&handle).unwrap();

        #[allow(irrefutable_let_patterns)]
        while let frames = consumer.recv() {
            let shutdown = shutdown.clone();
            let mut shutdown = shutdown.lock().unwrap();

            if shutdown.get_mut().to_owned() {
                break;
            }

            match frames {
                Ok(frames) => {
                    let mut interweaved_frame = Vec::<f32>::new();
                    for frame in frames.frames {
                        let _client_id = frame.client_id;
                        let mut pcm = SamplesBuffer::new(
                            config_c.channels,
                            config_c.sample_rate.0.into(),
                            frame.pcm.clone()
                        );

                        // Attenuate the SampleBuffer
                        let pcm = pcm.amplify(2.0);
                        // Check if the client is muted, and mute them (skip this block entirely)
                        // 3D Audio & Attenuate
                        // Attenuate based on individual audio setting

                        let attenuated_pcm: Vec<f32> = pcm.collect();
                        interweaved_frame = super::interweave(
                            interweaved_frame.as_mut(),
                            attenuated_pcm.as_ref()
                        );
                    }

                    let source = SamplesBuffer::new(
                        config_c.channels,
                        config_c.sample_rate.0.into(),
                        interweaved_frame.clone()
                    );
                    sink.append(source);
                }
                Err(_) => {}
            }
        }
        tracing::info!("Output stream ended.");
    });

    let rx = rx.clone();
    tokio::spawn(async move {
        let sample_rate: u32 = config.sample_rate.0.into();

        let mut decoders = HashMap::<Vec<u8>, Arc<Mutex<opus::Decoder>>>::new();
        #[allow(irrefutable_let_patterns)]
        while let packet = rx.recv() {
            match packet {
                Ok(packet) => {
                    let mut frames = Vec::new();
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
                                    frames.push(RawAudioFramePacket {
                                        client_id,
                                        pcm: out,
                                    });
                                }
                            }
                            Err(_) => {}
                        }
                    }

                    _ = producer.send(RawAudioFramePacketCollection {
                        frames,
                    });
                }
                Err(_) => {}
            }
        }
    });
    Ok(true)
}
