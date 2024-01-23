use std::sync::Arc;
use async_mutex::Mutex;
use common::structs::{
    audio::AudioDeviceType,
    config::StreamType,
    packet::{ AudioFramePacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData },
};
use rodio::cpal::{ BufferSize, traits::{ DeviceTrait, StreamTrait } };
use opus::Bitrate;
use audio_gate::NoiseGate;
use flume::Sender;
use rtrb::RingBuffer;
use tauri::State;

use std::sync::mpsc;

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
    super::stop_stream(StreamType::InputStream).await;

    // We're using a mpsc channel to transfer signals from the thread that passes message to the network stream
    // and the underlying stream itself
    let (mpsc_tx, mpsc_rx) = mpsc::channel();

    // A local Moka cache stores a self-assigned ID we store in this thread
    // If the thread needs to be canceled, we simply remove it and the loop checks if it's present or not.
    let (id, cache) = match super::setup_task_cache(super::INPUT_STREAM).await {
        Ok((id, cache)) => (id, cache),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let gamertag = match crate::invocations::credentials::get_credential("gamertag".into()).await {
        Ok(gt) => gt,
        Err(_) => {
            return Err(false);
        }
    };

    // The audio device we want to use.
    let device = match super::get_device(device, AudioDeviceType::InputDevice, None).await {
        Ok(device) => device,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let config: cpal::StreamConfig = match device.default_input_config() {
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
            let message: &'static str = mpsc_rx.recv().unwrap();
            if message.eq("terminate") {
                stream.pause().unwrap();
                break;
            }
        }

        tracing::info!("Dropped input stream.");
        drop(stream);
        return;
    });

    // This is our processing thread for our audio frames
    // Each frame is packaged into an opus frame, then sent to the network.rs to be submitted to the server
    let tx = tx.clone();
    let consumer = Arc::new(Mutex::new(consumer));
    tokio::spawn(async move {
        let sample_rate: u32 = config.sample_rate.0.into();
        let id = id.clone();
        let mut data_stream = Vec::<f32>::new();
        let mut encoder = match
            opus::Encoder::new(sample_rate, opus::Channels::Mono, opus::Application::Voip)
        {
            Ok(encoder) => encoder,
            Err(e) => {
                tracing::error!("Encoder failed to create: {}", e.to_string());
                return;
            }
        };

        _ = encoder.set_bitrate(Bitrate::Bits(64_000));
        tracing::info!("Opus Encoder Bitrate: {:?}", encoder.get_bitrate().ok());

        let consumer = consumer.clone();
        let mut consumer = consumer.lock_arc().await;
        loop {
            let sample = consumer.pop();
            match sample {
                Ok(mut sample) => {
                    let id = id.clone();

                    if
                        crate::invocations::should_self_terminate_sync(
                            &id,
                            &cache.clone(),
                            super::INPUT_STREAM
                        )
                    {
                        mpsc_tx.send("terminate").unwrap();
                        break;
                    }

                    data_stream.append(&mut sample);

                    // This should practically only ever fire once
                    // So this code is largely redundant.
                    // @todo!() measure if this is even necessary
                    if data_stream.len() >= (super::BUFFER_SIZE as usize) {
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
        tracing::info!("Input audio buffer sender ended.");
        return;
    });

    // Calling this from the client will result in an immediate return, but the threads will remain active.
    return Ok(true);
}
