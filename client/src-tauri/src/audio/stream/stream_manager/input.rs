use super::AudioFrame;
use crate::audio::types::{AudioDevice, BUFFER_SIZE};
use crate::{audio::stream::stream_manager::AudioFrameData, NetworkPacket};
use anyhow::anyhow;
use audio_gate::NoiseGate;
use common::structs::audio::NoiseGateSettings;
use common::structs::packet::{AudioFramePacket, QuicNetworkPacket, QuicNetworkPacketData};
use log::{error, warn};
use once_cell::sync::Lazy;
use opus::Bitrate;
use rodio::cpal::traits::StreamTrait as CpalStreamTrait;
use rodio::DeviceTrait;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::task::{AbortHandle, JoinHandle};

/// Indicator for if the Input Stream should be muted
/// If this i
static MUTE_INPUT_STREAM: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static USE_NOISE_GATE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static UPDATE_NOISE_GATE_SETTINGS: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static NOISE_GATE_SETTINGS: Lazy<Mutex<serde_json::Value>> = Lazy::new(|| {
    Mutex::new(
        serde_json::to_value(NoiseGateSettings::default())
            .expect("Failed to serialize NoiseGateSettings"),
    )
});

pub(crate) struct InputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Sender<NetworkPacket>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<moka::future::Cache<String, String>>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
}

impl common::traits::StreamTrait for InputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        log::info!("Setting metadata for input stream: {} = {}", key, value);
        match key.as_str() {
            // Toggle Mute
            "mute" => {
                self.mute();
            }
            // Toggle Noise Gate
            "use_noise_gate" => {
                match value.as_str() {
                    "true" => USE_NOISE_GATE.store(true, Ordering::Relaxed),
                    _ => USE_NOISE_GATE.store(false, Ordering::Relaxed),
                };
            }
            "noise_gate_settings" => {
                match serde_json::from_str::<NoiseGateSettings>(&value) {
                    Ok(settings) => {
                        let mut lock_settings = NOISE_GATE_SETTINGS.lock().unwrap();
                        *lock_settings = serde_json::to_value(settings)
                            .expect("Failed to serialize NoiseGateSettings");
                        UPDATE_NOISE_GATE_SETTINGS.store(true, Ordering::Relaxed);
                        drop(lock_settings);
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to deserialize NoiseGateSettings on metadata set: {}",
                            e
                        );
                    }
                };
            }
            _ => {
                let metadata = self.metadata.clone();
                metadata.insert(key, value).await;
            }
        };

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

    fn is_stopped(&self) -> bool {
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
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            device,
            bus,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata,
            app_handle: app_handle.clone(),
        }
    }

    // Produces raw PCM data and sends it to the network consumer
    fn listener(
        &mut self,
        producer: SyncSender<AudioFrame>,
        shutdown: Arc<AtomicBool>,
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
                                buffer_size: cpal::BufferSize::Fixed(
                                    crate::audio::types::BUFFER_SIZE,
                                ),
                            };

                            let settings = NOISE_GATE_SETTINGS.lock().unwrap();
                            let noise_gate_settings =
                                match serde_json::from_value::<NoiseGateSettings>(settings.clone())
                                {
                                    Ok(settings) => settings,
                                    Err(_) => NoiseGateSettings::default(),
                                };

                            let mut gate = NoiseGate::new(
                                noise_gate_settings.open_threshold,
                                noise_gate_settings.close_threshold,
                                device_config.sample_rate.0 as f32,
                                match device_config.channels.into() {
                                    1 => 1,
                                    2 => 2,
                                    _ => 2,
                                },
                                noise_gate_settings.release_rate,
                                noise_gate_settings.attack_rate,
                                noise_gate_settings.hold_time,
                            );

                            drop(settings);

                            let error_fn = move |error| {
                                warn!("an error occured on the stream: {}", error);
                            };

                            let mut process_fn = move |data: &[f32]| {
                                let pcm: Vec<f32>;
                                // If the noise gate is enabled, process data through it
                                if USE_NOISE_GATE.load(Ordering::Relaxed) {
                                    // If there is a pending update, apply it, then disable the lock check
                                    if UPDATE_NOISE_GATE_SETTINGS.load(Ordering::Relaxed) {
                                        let current_settings = NOISE_GATE_SETTINGS.lock().unwrap();
                                        match serde_json::from_value::<NoiseGateSettings>(
                                            current_settings.clone(),
                                        ) {
                                            Ok(settings) => {
                                                log::info!(
                                                    "Updating noise gate settings: {:?}",
                                                    settings
                                                );
                                                gate.update(
                                                    settings.open_threshold,
                                                    settings.close_threshold,
                                                    settings.release_rate,
                                                    settings.attack_rate,
                                                    settings.hold_time,
                                                );
                                            }
                                            Err(e) => {
                                                warn!("Noise gate settings were asked to update, but failed to deserialize: {}", e);
                                            }
                                        };
                                        drop(current_settings);
                                        // Even if we fail to update the noise gate settings, clear the lock
                                        UPDATE_NOISE_GATE_SETTINGS.store(false, Ordering::Relaxed);
                                    }

                                    // Process the frame through the gate
                                    pcm = gate.process_frame(&data);
                                } else {
                                    // Send data through the gate normally
                                    pcm = data.to_vec();
                                }
                                let pcm_sendable = pcm.iter().all(|&e| f32::abs(e) == 0.0);

                                // Only send audio frame data if our filters haven't cut data out
                                if !pcm_sendable && !MUTE_INPUT_STREAM.load(Ordering::Relaxed) {
                                    let audio_frame_data = AudioFrameData { pcm };

                                    match producer.try_send(AudioFrame::F32(audio_frame_data)) {
                                        Ok(()) => {}
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
                                    None,
                                ),
                                rodio::cpal::SampleFormat::I32 => cpal_device.build_input_stream(
                                    &device_config,
                                    move |data: &[i32], _: &cpal::InputCallbackInfo| {
                                        const SCALE: f32 = 2147483648.0; // 2^31 to normalize properly
                                        let f32_data: Vec<f32> = data
                                            .iter()
                                            .map(|&sample| sample as f32 / SCALE)
                                            .collect();
                                        process_fn(&f32_data);
                                    },
                                    error_fn,
                                    None,
                                ),
                                rodio::cpal::SampleFormat::I16 => cpal_device.build_input_stream(
                                    &device_config,
                                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                        const SCALE: f32 = 32768.0; // 2^15 to normalize properly
                                        let f32_data: Vec<f32> = data
                                            .iter()
                                            .map(|&sample| sample as f32 / SCALE)
                                            .collect();
                                        process_fn(&f32_data);
                                    },
                                    error_fn,
                                    None,
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

                                    warn!(
                                        "{} {} ended.",
                                        device.io.to_string(),
                                        device.display_name
                                    );
                                    drop(stream);
                                    return;
                                }
                                Err(e) => {
                                    error!("{:?}", e);
                                }
                            };
                        }),
                        None => {
                            error!("CPAL output device is not defined. This shouldn't happen! Restart BVC? {:?}", device.clone());
                            return Err(anyhow::anyhow!(
                                "Couldn't retrieve native cpal device for {} {}.",
                                device.io.to_string(),
                                device.display_name
                            ));
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
        shutdown: Arc<AtomicBool>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => {
                match device.get_stream_config() {
                    Ok(config) => {
                        let device_config = rodio::cpal::StreamConfig {
                            channels: match config.channels() {
                                1 => 1,
                                2 => 2,
                                _ => 1,
                            },
                            sample_rate: config.sample_rate(),
                            buffer_size: cpal::BufferSize::Fixed(crate::audio::types::BUFFER_SIZE),
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
                            #[cfg(target_os = "windows")]
                            {
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
                                            None => continue,
                                        };

                                        data_stream.append(&mut raw_sample);
                                        while data_stream.len() >= (BUFFER_SIZE as usize * 4) {
                                            let sample_to_process: Vec<f32> = data_stream
                                                .drain(0..BUFFER_SIZE as usize)
                                                .collect();

                                            let encoded_data = match encoder.encode_vec_float(
                                                &sample_to_process,
                                                sample_to_process.len() * 4,
                                            ) {
                                                Ok(s) if s.len() > 3 => s,
                                                _ => continue, // Skip if encoding failed or insufficient data
                                            };

                                            let packet = NetworkPacket {
                                            data: QuicNetworkPacket {
                                                packet_type: common::structs::packet::PacketType::AudioFrame,
                                                owner: None, // This will be populated on the network side
                                                data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                                                    encoded_data.clone(),
                                                    device_config.sample_rate.0,
                                                    None,
                                                    None,
                                                    None,
                                                    None
                                                ))
                                            }
                                        };

                                            if let Err(e) = tx.send_async(packet).await {
                                                error!("Sending audio frame to Quic network thread failed: {:?}", e);
                                            } else {
                                                notify.notify_waiters();
                                                notify.notified().await;
                                            }
                                        }
                                    }
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
                }
            }
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
