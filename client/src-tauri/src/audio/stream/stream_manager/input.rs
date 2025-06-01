use anyhow::anyhow;
use audio_gate::NoiseGate;
use common::structs::{
    audio::{AudioDevice, BUFFER_SIZE},
    packet::{AudioFramePacket, QuicNetworkPacket, QuicNetworkPacketData},
};
use log::{error, warn};
use opus::Bitrate;
use rodio::cpal::traits::StreamTrait;
use rodio::DeviceTrait;
use std::{sync::{
    atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, SyncSender}, Arc
}, time::Duration};
use tokio::task::{AbortHandle, JoinHandle};
use crate::{audio::stream::stream_manager::AudioFrameData, NetworkPacket};
use super::AudioFrame;
use log::info;
use once_cell::sync::Lazy;

static MUTE_INPUT_STREAM: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub(crate) struct InputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Sender<NetworkPacket>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<moka::future::Cache<String, String>>,
    app_handle: tauri::AppHandle
}

impl super::StreamTrait for InputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        let metadata = self.metadata.clone();
        metadata.insert(key, value).await;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(true, Ordering::Relaxed);

        _ = tokio::time::sleep(Duration::from_millis(500)).await;

        // Then hard terminate them
        for job in &self.jobs {
            job.abort();
        }

        self.jobs = vec![];
        Ok(())
    }

    fn is_stopped(&mut self) -> bool {
        self.jobs.len() == 0
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);
        
        let mut jobs = vec![];

        let (producer, consumer) = mpsc::sync_channel(1000);

        // Start the audio input listener thread
        match self.listener(producer, self.shutdown.clone()) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input listener encountered an error: {:?}", e);
                return Err(e);
            }
        };

        // Send the PCM data to the network sender
        match self.sender(consumer, self.shutdown.clone()) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input sender encountered an error: {:?}", e);
                return Err(e);
            }
        };

        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();
        Ok(())
    }
}

impl InputStream {
    pub fn new(
        device: Option<AudioDevice>,
        bus: Arc<flume::Sender<NetworkPacket>>,
        metadata: Arc<moka::future::Cache<String, String>>,
        app_handle: tauri::AppHandle
    ) -> Self {
        Self {
            device,
            bus,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata,
            app_handle: app_handle.clone()
        }
    }

    // Produces raw PCM data and sends it to the network consumer
    fn listener(
        &mut self,
        producer: SyncSender<AudioFrame>,
        shutdown: Arc<AtomicBool>
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let cpal_device: Option<rodio::cpal::Device> = device.clone().into();

                    let handle = match cpal_device {
                        Some(cpal_device) => tokio::spawn(async move {
                            let cpal_device = cpal_device.clone();
                            let device = device.clone();
                            let device_config = rodio::cpal::StreamConfig {
                                channels: config.channels(),
                                sample_rate: config.sample_rate(),
                                buffer_size: cpal::BufferSize::Fixed(common::structs::audio::BUFFER_SIZE)
                            };

                            // @todo: Move this out of this thread and let it be configurable
                            let mut gate = NoiseGate::new(
                                -36.0,
                                -56.0,
                                device_config.sample_rate.0 as f32,
                                device_config.channels.into(), // Todo: this should either be 1 or 2, not +++
                                150.0,
                                5.0,
                                150.0
                            );

                            let error_fn = move | error | {
                                warn!("an error occured on the stream: {}", error);
                            };

                            let mut process_fn = move | data: &[f32] | {
                                // Gate
                                let pcm = gate.process_frame(&data);
                                let pcm_sendable = pcm.iter().all(|&e| f32::abs(e) == 0.0);

                                // Only send audio frame data if our filters haven't cut data out
                                if !pcm_sendable && !MUTE_INPUT_STREAM.load(Ordering::Relaxed) {
                                    let audio_frame_data = AudioFrameData { pcm };

                                    match producer.try_send(AudioFrame::F32(audio_frame_data)) {
                                        Ok(()) => {},
                                        Err(_e) => {}
                                    }
                                }
                            };

                            let stream = match config.sample_format() {
                                rodio::cpal::SampleFormat::F32 => cpal_device.build_input_stream(
                                    &device_config,
                                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                        process_fn(&data);
                                    },
                                    error_fn,
                                    None
                                ),
                                rodio::cpal::SampleFormat::I32 => cpal_device.build_input_stream(
                                    &device_config,
                                    move |data: &[i32], _: &cpal::InputCallbackInfo| {
                                        const SCALE: f32 = 2147483648.0; // 2^31 to normalize properly
                                        let f32_data: Vec<f32> = data.iter().map(|&sample| sample as f32 / SCALE).collect();
                                        process_fn(&f32_data);
                                    },
                                    error_fn,
                                    None
                                ),
                                rodio::cpal::SampleFormat::I16 => cpal_device.build_input_stream(
                                    &device_config,
                                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                        const SCALE: f32 = 32768.0; // 2^15 to normalize properly
                                        let f32_data: Vec<f32> = data.iter().map(|&sample| sample as f32 / SCALE).collect();
                                        process_fn(&f32_data);
                                    },
                                    error_fn,
                                    None
                                ),
                                _ => {
                                    error!("{} {} does not have a supported sample format for streaming.", device.io.to_string(), device.display_name);
                                    return;
                                }
                            };

                            match stream {
                                Ok(stream) => {
                                    _ = stream.play().unwrap();
                                    
                                    // Get the current thread handle
                                    let thread = std::thread::current();
                                    
                                    // Spawn a thread that will unpark when shutdown is triggered
                                    let shutdown_clone = shutdown.clone();
                                    let thread_clone = thread.clone();
                                    std::thread::spawn(move || {
                                        while !shutdown_clone.load(Ordering::Relaxed) {
                                            std::thread::sleep(Duration::from_millis(100));
                                        }
                                        thread_clone.unpark();
                                    });
                                    
                                    // Park the current thread until shutdown
                                    std::thread::park();
                                    
                                    stream.pause().unwrap();

                                    warn!("{} {} ended.", device.io.to_string(), device.display_name);
                                    drop(stream);
                                    return;
                                },
                                Err(e) => {
                                    error!("{:?}", e);
                                }
                            };
                        }),
                        None => {
                            error!("CPAL output device is not defined. This shouldn't happen! Restart BVC? {:?}", device.clone());
                            return Err(anyhow::anyhow!("Couldn't retrieve native cpal device for {} {}.", device.io.to_string(), device.display_name))
                        }
                    };

                    return Ok(handle);
                }
                Err(e) => return Err(e),
            },
            None => {
                return Err(anyhow!(
                    "InputStream is not initialized with a device! Unable to start stream"
                ))
            }
        };
    }

    fn sender(
        &mut self,
        consumer: Receiver<AudioFrame>,
        shutdown: Arc<AtomicBool>
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let device_config = rodio::cpal::StreamConfig {
                        channels: config.channels(),
                        sample_rate: config.sample_rate(),
                        buffer_size: cpal::BufferSize::Fixed(common::structs::audio::BUFFER_SIZE),
                    };

                    let mut data_stream = Vec::<f32>::new();

                    // Create the opus encoder
                    let mut encoder = match opus::Encoder::new(
                        device_config.sample_rate.0.into(),
                        opus::Channels::Mono,
                        opus::Application::Voip,
                    ) {
                        Ok(mut encoder) => {
                            _ = encoder.set_bitrate(Bitrate::Bits(32_000));
                            encoder
                        }
                        Err(e) => {
                            error!("Could not create opus encoder: {}", e.to_string());
                            return Err(anyhow!("{}", e.to_string()));
                        }
                    };

                    let bus = self.bus.clone();
                    let notify = crate::AUDIO_INPUT_NETWORK_NOTIFY.clone();
                    let handle = tokio::spawn(async move {
                        #[cfg(target_os = "windows")] {
                            windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
                            unsafe {
                                timeBeginPeriod(1);
                            }
                        }
                        let tx = bus.clone();
                        #[allow(irrefutable_let_patterns)]
                        while let sample = consumer.recv() {
                            if shutdown.load(Ordering::Relaxed) {
                                warn!("Audio Input stream, quic sender received shutdown signal, and is now terminating.");
                                break;
                            }
                            
                            
                            match sample {
                                Ok(sample) => {
                                    let mut raw_sample = match sample.f32() {
                                        Some(sample) => sample.pcm,
                                        None => continue
                                    };

                                    data_stream.append(&mut raw_sample);
                                    while data_stream.len() >= (BUFFER_SIZE as usize * 4) {
                                        let sample_to_process: Vec<f32> = data_stream
                                            .drain(0..BUFFER_SIZE as usize)
                                            .collect();

                                        let encoded_data = match encoder.encode_vec_float(&sample_to_process, sample_to_process.len() * 4) {
                                            Ok(s) if s.len() > 3 => s,
                                            _ => continue, // Skip if encoding failed or insufficient data
                                        };

                                        let packet = NetworkPacket {
                                            data: QuicNetworkPacket {
                                                packet_type: common::structs::packet::PacketType::AudioFrame,
                                                owner: None, // This will be populated on the network side
                                                data: QuicNetworkPacketData::AudioFrame(AudioFramePacket {
                                                    length: encoded_data.len(),
                                                    data: encoded_data.clone(),
                                                    sample_rate: device_config.sample_rate.0,
                                                    coordinate: None,
                                                    orientation: None,
                                                    dimension: None,
                                                    spatial: None
                                                })
                                            }
                                        };

                                        if let Err(e) = tx.send_async(packet).await {
                                            error!("Sending audio frame to Quic network thread failed: {:?}", e);
                                        } else {
                                            notify.notify_waiters();
                                            notify.notified().await;
                                        }
                                    }
                                },
                                Err(_e) => {
                                    // We're intentionaly supressing this error because the channel could not
                                    // yet be established -- since this is syncronous this'll throw constantly otherwise
                                }
                            }
                        }
                    });

                    return Ok(handle);
                }
                Err(e) => return Err(e),
            },
            None => {
                return Err(anyhow!(
                    "InputStream is not initialized with a device! Unable to start stream"
                ))
            }
        };
    }

    pub fn mute(&self) {
        let current_state = MUTE_INPUT_STREAM.load(Ordering::Relaxed);
        MUTE_INPUT_STREAM.store(!current_state, Ordering::Relaxed);
    }

    pub fn mute_status(&self) -> bool {
        MUTE_INPUT_STREAM.load(Ordering::Relaxed)
    }
}
