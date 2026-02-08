use super::resampler::AudioResampler;

use super::AudioFrame;
use crate::audio::recording::{RawRecordingData, RecordingProducer};
use crate::audio::stream::{RecoverySender, StreamRecoveryEvent};
use crate::audio::types::{AudioDevice, AudioDeviceCpal, AudioDeviceType, BUFFER_SIZE};
use crate::{audio::stream::stream_manager::AudioFrameData, NetworkPacket};
use anyhow::anyhow;
use audio_gate::NoiseGate;
use common::structs::audio::{NoiseGateSettings, StreamEvent};
use common::structs::packet::{AudioFramePacket, QuicNetworkPacket, QuicNetworkPacketData};
use common::RecordingPlayerData;
use log::{error, debug, warn};
use once_cell::sync::Lazy;
use opus2::Bitrate;
use rodio::cpal::traits::StreamTrait as CpalStreamTrait;
use rodio::DeviceTrait;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri_plugin_store::StoreExt;
use tokio::task::{AbortHandle, JoinHandle};

/// Indicator for if the Input Stream should be muted
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
    recording_producer: Option<Arc<RecordingProducer>>,
    recording_active: Option<Arc<AtomicBool>>,  // Shared from RecordingManager
    recovery_tx: RecoverySender,
}

impl common::traits::StreamTrait for InputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        log::info!("Setting metadata for input stream: {} = {}", key, value);
        match key.as_str() {
            // Toggle Mute
            "mute" => {
                self.toggle(StreamEvent::Mute);
            },
            "record" => {
                // Recording is now controlled by RecordingManager's shared flag
                // No action needed here
            },
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

        let (producer, consumer) = flume::bounded::<AudioFrame>(1000);

        // Get current player name from store before starting (fail fast if not set)
        let store = self.app_handle.store("store.json")
            .map_err(|e| anyhow!("Failed to access store: {}", e))?;

        let current_player_name = store.get("current_player")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("Cannot start input stream without current_player set in store"))?;

        // Start the audio input listener thread
        match self.listener(producer, self.shutdown.clone()) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input listener encountered an error: {:?}", e);
                return Err(e);
            }
        };

        // Send the PCM data to the network sender
        match self.sender(consumer, self.shutdown.clone(), current_player_name, self.recording_active.clone()) {
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
        recording_producer: Option<Arc<RecordingProducer>>,
        recording_active: Option<Arc<AtomicBool>>,
        recovery_tx: RecoverySender,
    ) -> Self {
        Self {
            device,
            bus,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata,
            app_handle: app_handle.clone(),
            recording_producer,
            recording_active,
            recovery_tx,
        }
    }

    // Produces raw PCM data and sends it to the network consumer
    fn listener(
        &mut self,
        producer: flume::Sender<AudioFrame>,
        shutdown: Arc<AtomicBool>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        // Clone recovery_tx for use in the async block
        let recovery_tx = self.recovery_tx.clone();

        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(stored_config) => {
                    // Validate stored config against live device - detect Windows sound settings changes
                    let config = match crate::audio::device::refresh_device_config(&device) {
                        Some(fresh_configs) if !fresh_configs.is_empty() => {
                            let fresh_config: rodio::cpal::SupportedStreamConfig =
                                fresh_configs[0].clone().into();
                            if fresh_config.sample_rate() != stored_config.sample_rate() {
                                warn!(
                                    "Device {} sample rate changed: stored {}Hz, actual {}Hz. Using actual.",
                                    device.display_name,
                                    stored_config.sample_rate().0,
                                    fresh_config.sample_rate().0
                                );
                            }
                            fresh_config
                        }
                        _ => {
                            warn!("Could not refresh device config for {}, using stored config", device.display_name);
                            stored_config
                        }
                    };

                    let cpal_device = device.clone().to_cpal_device();

                    let handle = match cpal_device {
                        Some(cpal_device) => tokio::spawn(async move {
                            // Clone for error handling
                            let recovery_tx_for_error = recovery_tx.clone();
                            let shutdown_for_error = shutdown.clone();
                            /// CoreAudio on iOS should use the default buffer size to
                            /// Otherwise the InputStream fails to initialize
                            #[cfg(target_os = "ios")]
                            let buffer_size = cpal::BufferSize::Default;

                            #[cfg(not(target_os = "ios"))]
                            let buffer_size = cpal::BufferSize::Fixed(crate::audio::types::BUFFER_SIZE);

                            let cpal_device = cpal_device.clone();
                            let device = device.clone();
                            let device_config = rodio::cpal::StreamConfig {
                                channels: config.channels(),
                                sample_rate: config.sample_rate(),
                                buffer_size,
                            };

                            log::info!("Stream Config: {:?} {:?}", config.channels(), config.sample_rate());

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

                            // Error callback - signals shutdown and triggers recovery
                            let error_fn = move |error: cpal::StreamError| {
                                error!("Audio stream error (device may have disconnected): {}", error);
                                shutdown_for_error.store(true, Ordering::Relaxed);
                                // Signal recovery (thread-safe, non-blocking send)
                                let _ = recovery_tx_for_error.send(StreamRecoveryEvent::DeviceError {
                                    device_type: AudioDeviceType::InputDevice,
                                    error: error.to_string(),
                                });
                            };

                            let mut callback_count = 0u64;
                            let mut sent_count = 0u64;

                            // Pre-allocate buffer for in-place processing (max stereo size)
                            let mut pcm_buffer: Vec<f32> = vec![0.0; 960 * 2];

                            // Create resampler if device sample rate is not 48 kHz
                            let mut audio_resampler = match AudioResampler::new_if_needed(device_config.sample_rate.0) {
                                Some(Ok(r)) => {
                                    warn!(
                                        "Input device sample rate {} Hz requires resampling to 48 kHz. \
                                         For optimal performance, use a device that supports 48 kHz natively.",
                                        device_config.sample_rate.0
                                    );
                                    Some(r)
                                }
                                Some(Err(e)) => {
                                    error!("Failed to create audio resampler: {:?}", e);
                                    None  // Fall through without resampling - will likely cause Opus errors
                                }
                                None => None,  // Already 48 kHz, no resampling needed
                            };

                            let mut process_fn = move |data: &[f32]| {
                                callback_count += 1;
                                if callback_count == 1 || callback_count % 100 == 0 {
                                    debug!("[INPUT] CPAL callback #{}, sent {} frames so far", callback_count, sent_count);
                                }

                                let len = data.len();
                                pcm_buffer[..len].copy_from_slice(data);

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

                                    // Process the frame in-place through the gate
                                    gate.process_frame(&mut pcm_buffer[..len]);
                                }

                                // Convert to mono if stereo (single allocation for channel transfer)
                                let mono_pcm: Vec<f32> = if device_config.channels == 2 {
                                    pcm_buffer[..len].chunks_exact(2)
                                        .map(|lr| (lr[0] + lr[1]) / 2.0)
                                        .collect()
                                } else {
                                    pcm_buffer[..len].to_vec()
                                };

                                // Resample 44.1 kHz → 48 kHz if needed
                                let mono_pcm = if let Some(ref mut rs) = audio_resampler {
                                    rs.process(&mono_pcm)
                                } else {
                                    mono_pcm
                                };

                                let pcm_sendable = mono_pcm.iter().all(|&e| f32::abs(e) == 0.0);

                                // Only send audio frame data if our filters haven't cut data out
                                if !pcm_sendable && !MUTE_INPUT_STREAM.load(Ordering::Relaxed) {
                                    // Capture timestamp at audio capture time for accurate recording timecode
                                    let captured_at_ms = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64;

                                    let audio_frame_data = AudioFrameData {
                                        pcm: mono_pcm,
                                        captured_at_ms,
                                    };

                                    match producer.try_send(AudioFrame::F32(audio_frame_data)) {
                                        Ok(()) => {
                                            sent_count += 1;
                                        }
                                        Err(_e) => {}
                                    }
                                } else if callback_count % 100 == 0 {
                                    debug!("[INPUT] Skipping send - pcm_sendable={}, muted={}",
                                          pcm_sendable,
                                          MUTE_INPUT_STREAM.load(Ordering::Relaxed));
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
                                    // Start the stream with proper error handling
                                    if let Err(e) = stream.play() {
                                        error!("Failed to start audio stream: {:?}", e);
                                        shutdown.store(true, Ordering::Relaxed);
                                        let _ = recovery_tx.send(StreamRecoveryEvent::DeviceError {
                                            device_type: AudioDeviceType::InputDevice,
                                            error: format!("Failed to start stream: {:?}", e),
                                        });
                                        return;
                                    }

                                    // Get the current thread handle
                                    let thread = std::thread::current();

                                    // Spawn a thread that will unpark when shutdown is triggered
                                    let shutdown_clone = shutdown.clone();
                                    let thread_clone = thread.clone();
                                    std::thread::spawn(move || {
                                        while !shutdown_clone.load(Ordering::Relaxed) {
                                            std::thread::sleep(Duration::from_millis(500));
                                        }
                                        thread_clone.unpark();
                                    });

                                    // Park the current thread until shutdown
                                    std::thread::park();

                                    // Pause the stream (may fail if device already disconnected)
                                    if let Err(e) = stream.pause() {
                                        warn!("Failed to pause stream (may already be stopped): {:?}", e);
                                    }

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
        consumer: flume::Receiver<AudioFrame>,
        shutdown: Arc<AtomicBool>,
        current_player_name: String,
        recording_active: Option<Arc<AtomicBool>>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => {
                match device.get_stream_config() {
                    Ok(stored_config) => {
                        // Validate stored config against live device
                        let config = match crate::audio::device::refresh_device_config(&device) {
                            Some(fresh_configs) if !fresh_configs.is_empty() => {
                                let fresh_config: rodio::cpal::SupportedStreamConfig =
                                    fresh_configs[0].clone().into();
                                fresh_config
                            }
                            _ => stored_config,
                        };
                        /// CoreAudio on iOS should use the default buffer size to
                        /// Otherwise the InputStream fails to initialize
                        #[cfg(target_os = "ios")]
                        let buffer_size = cpal::BufferSize::Default;

                        #[cfg(not(target_os = "ios"))]
                        let buffer_size = cpal::BufferSize::Fixed(crate::audio::types::BUFFER_SIZE);

                        let original_sample_rate = config.sample_rate();

                        // Force 48 kHz if device was not 48 kHz (already resampled in listener)
                        let effective_sample_rate = if original_sample_rate.0 != crate::audio::types::OPUS_SAMPLE_RATE {
                            rodio::cpal::SampleRate(crate::audio::types::OPUS_SAMPLE_RATE)
                        } else {
                            original_sample_rate
                        };

                        let device_config = rodio::cpal::StreamConfig {
                            channels: match config.channels() {
                                1 => 1,
                                2 => 2,
                                _ => 1,
                            },
                            sample_rate: effective_sample_rate,
                            buffer_size,
                        };

                        let mut data_stream = Vec::<f32>::new();

                        // Create the opus encoder
                        let mut encoder = match opus2::Encoder::new(
                            device_config.sample_rate.0.into(),
                            opus2::Channels::Mono,
                            opus2::Application::Voip,
                        ) {
                            Ok(mut encoder) => {
                                _ = encoder.set_bitrate(Bitrate::Bits(32_000));

                                // Lower complexity on mobile for battery/heat savings
                                #[cfg(any(target_os = "android", target_os = "ios"))]
                                {
                                    _ = encoder.set_complexity(7);
                                }

                                encoder
                            }
                            Err(e) => {
                                error!("Could not create opus encoder: {}", e.to_string());
                                return Err(anyhow!("{}", e.to_string()));
                            }
                        };

                        let bus = self.bus.clone();
                        let recording_producer = self.recording_producer.clone();

                        let handle = tokio::spawn(async move {
                            #[cfg(target_os = "windows")]
                            {
                                windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
                                unsafe {
                                    timeBeginPeriod(1);
                                }
                            }
                            let tx = bus.clone();

                            let mut first_sample_timestamp_ms: Option<u64> = None;

                            #[allow(irrefutable_let_patterns)]
                            while let Ok(sample) = consumer.recv_async().await {
                                if shutdown.load(Ordering::Relaxed) {
                                    warn!("Audio Input stream, quic sender received shutdown signal, and is now terminating.");
                                    break;
                                }

                                let sample_data = match sample.f32() {
                                    Some(sample) => sample,
                                    None => continue,
                                };

                                if data_stream.is_empty() {
                                    first_sample_timestamp_ms = Some(sample_data.captured_at_ms);
                                }

                                let mut raw_sample = sample_data.pcm;

                                data_stream.append(&mut raw_sample);
                                while data_stream.len() >= BUFFER_SIZE as usize {
                                    let sample_to_process: Vec<f32> = data_stream
                                        .drain(0..BUFFER_SIZE as usize)
                                        .collect();

                                    let encoded_data = match encoder.encode_vec_float(
                                        &sample_to_process,
                                        sample_to_process.len() * 4,
                                    ) {
                                        Ok(s) if s.len() > 3 => s,
                                        _ => continue,
                                    };

                                    // Check shared recording flag from RecordingManager
                                    if let Some(ref flag) = recording_active {
                                        if flag.load(Ordering::SeqCst) {
                                            if let Some(ref producer) = recording_producer {
                                                // Use the timestamp from when the first sample was captured
                                                // This ensures the recording timestamp matches actual capture time
                                                let recording_data = RawRecordingData::InputData {
                                                    absolute_timestamp_ms: first_sample_timestamp_ms,
                                                    opus_data: encoded_data.clone(),
                                                    sample_rate: device_config.sample_rate.0,
                                                    channels: device_config.channels.into(),
                                                    emitter: RecordingPlayerData::for_input(
                                                        current_player_name.clone(),
                                                        None, // TODO: Add gain cache for input
                                                    ),
                                                };

                                                let _ = producer.try_send(recording_data);
                                            }
                                        }
                                    }

                                    // Reset first sample timestamp after processing this buffer
                                    // The next buffer will get a new timestamp from the first sample
                                    if data_stream.is_empty() {
                                        first_sample_timestamp_ms = None;
                                    }

                                    let packet = NetworkPacket {
                                        data: QuicNetworkPacket {
                                            packet_type: common::structs::packet::PacketType::AudioFrame,
                                            owner: None, // This will be populated on the network side
                                            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                                                encoded_data.clone(),
                                                device_config.sample_rate.0,
                                                None,  // sender (enriched by server)
                                                None   // spatial
                                            ))
                                        }
                                    };

                                    if let Err(e) = tx.send_async(packet).await {
                                        error!("Sending audio frame to Quic network thread failed: {:?}", e);
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

    pub fn toggle(&self, event: StreamEvent) {
        match event {
            StreamEvent::Mute => {
                let current_state = MUTE_INPUT_STREAM.load(Ordering::Relaxed);
                MUTE_INPUT_STREAM.store(!current_state, Ordering::Relaxed);
            },
            StreamEvent::Record => {
                // Recording state is now owned by RecordingManager
                // Streams read the shared flag directly - no toggle needed
            }
        }
    }

    pub fn mute_status(&self) -> bool {
        MUTE_INPUT_STREAM.load(Ordering::Relaxed)
    }
}
