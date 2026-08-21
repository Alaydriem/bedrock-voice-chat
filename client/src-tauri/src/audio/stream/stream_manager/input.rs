use super::resampler::AudioResampler;

use super::AudioFrame;
use super::{DeviceLease, JobSet};
use super::input_core::InputProcessCore;
use super::source::{AudioInputSource, CaptureConfig};
use crate::NetworkPacket;
use crate::audio::recording::{RawRecordingData, RecordingProducer};
use crate::audio::stream::RecoverySender;
use crate::audio::{AudioDevice, BUFFER_SIZE};
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::BedrockPlayerStateCache;
use anyhow::anyhow;
use audio_gate::NoiseGate;
use common::RecordingPlayerData;
use common::consts::OPUS_FRAME_DURATION_MS;
use common::structs::audio::{InputLevel, NoiseGateSettings, StreamEvent};
use common::structs::packet::{AudioFramePacket, QuicNetworkPacket, QuicNetworkPacketData};
use log::{error, warn};
use once_cell::sync::Lazy;
use opus2::Bitrate;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri_plugin_store::StoreExt;
use tokio::task::JoinHandle;

/// Indicator for if the Input Stream should be muted
pub(crate) static MUTE_INPUT_STREAM: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
pub(crate) static USE_NOISE_GATE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
pub(crate) static UPDATE_NOISE_GATE_SETTINGS: Lazy<AtomicBool> =
    Lazy::new(|| AtomicBool::new(false));
pub(crate) static NOISE_GATE_SETTINGS: Lazy<Mutex<serde_json::Value>> = Lazy::new(|| {
    Mutex::new(
        serde_json::to_value(NoiseGateSettings::default())
            .expect("Failed to serialize NoiseGateSettings"),
    )
});

pub(crate) struct InputStream {
    pub device: Option<AudioDevice>,
    source: AudioInputSource,
    pub bus: Arc<flume::Sender<NetworkPacket>>,
    jobs: JobSet,
    // Whether a session stream is meant to be capturing right now, which is the only thing that
    // makes an absent frame count a fault rather than an idle client. `jobs` cannot answer it:
    // its handles outlive a capture callback that stopped being called, which is exactly the
    // failure the watchdog exists to catch.
    capture_expected: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    stream: DeviceLease<rodio::cpal::Stream>,
    pub metadata: Arc<moka::future::Cache<String, String>>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
    recording_producer: Option<Arc<RecordingProducer>>,
    recording_active: Option<Arc<AtomicBool>>,
    recovery_tx: RecoverySender,
    input_stats: Arc<crate::diagnostics::InputPipelineStats>,
    // Where this microphone's level goes in a session, and the only thing that publishes it.
    levels: Arc<crate::audio::stream::level_bus::LevelBus>,
    // Whether to also publish the unquantised capture level at capture rate. True only for
    // the setup screen's metering stream; see `listener`.
    raw_levels: bool,
    #[cfg(feature = "bedrock-protocol")]
    player_state_cache: Option<Arc<BedrockPlayerStateCache>>,
}

impl common::traits::StreamTrait for InputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        log::info!("Setting metadata for input stream: {} = {}", key, value);
        match key.as_str() {
            // Toggle Mute
            "mute" => {
                self.toggle(StreamEvent::Mute);
            }
            "record" => {
                // Recording is now controlled by RecordingManager's shared flag
                // No action needed here
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
        if self.jobs.is_empty() && !self.stream.is_held() {
            return Ok(());
        }

        // Cleared first. A stop that raced the watchdog's read would otherwise be seen as a
        // stream that stopped delivering, and answered with a restart of the stream the caller
        // is in the middle of shutting down.
        self.capture_expected.store(false, Ordering::Relaxed);

        _ = self.shutdown.store(true, Ordering::Relaxed);

        // Before the join, not after: the capture callback owns the only sender into the frame
        // channel, so the sender job's `recv_async` ends when this drops.
        self.stream.release().await;

        if !self.jobs.settle(Self::STOP_GRACE).await {
            warn!("Input stream jobs did not finish within the grace window; aborting them.");
        }

        Ok(())
    }

    fn is_stopped(&self) -> bool {
        // Both, because either can be the only thing running: the session stream has a sender task
        // and a device, the setup screen's metering stream has only a device.
        self.jobs.is_empty() && !self.stream.is_held()
    }

    #[tracing::instrument(skip(self))]
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);
        self.raw_levels = false;

        let mut jobs = vec![];

        let (producer, consumer) = flume::bounded::<AudioFrame>(1000);

        // Get current player name from store before starting (fail fast if not set)
        let store = self
            .app_handle
            .store("store.json")
            .map_err(|e| anyhow!("Failed to access store: {}", e))?;

        let current_player_name = store
            .get("current_player")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| {
                anyhow!("Cannot start input stream without current_player set in store")
            })?;

        // Resolve capture parameters once for the active source variant. Both
        // the listener (gate/resampler/core) and the sender (Opus StreamConfig)
        // derive from this, so neither branches on the source kind.
        let capture_config = self.source.resolve_config(&self.device)?;
        let source_sample_rate = capture_config.sample_rate;
        let source_channels = capture_config.channels;

        // Open the capture device. The cpal callback is driven by the device rather than by a
        // task of ours, so this contributes no job — it leaves the stream on `self.stream`.
        match self.listener(capture_config, producer, self.shutdown.clone()) {
            Ok(stream) => self.stream.hold(stream).await,
            Err(e) => {
                curia::error!("input listener failed to start", {
                    defect: crate::logging::Defect::AudioDeviceLost,
                    io: "input",
                    error: e.to_string(),
                });
                return Err(e);
            }
        };

        // Send the PCM data to the network sender
        match self.sender(
            consumer,
            self.shutdown.clone(),
            current_player_name,
            self.recording_active.clone(),
            source_sample_rate,
            source_channels,
        ) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                curia::error!("input sender failed to start", {
                    defect: crate::logging::Defect::AudioDeviceLost,
                    io: "input",
                    error: e.to_string(),
                });
                return Err(e);
            }
        };

        self.jobs = JobSet::from(jobs);
        // Last, so the watchdog only ever arms over a stream that reached the end of start().
        self.capture_expected.store(true, Ordering::Relaxed);
        Ok(())
    }
}

impl InputStream {
    /// How long a stop waits for its own jobs before killing them. A backstop, not a schedule.
    const STOP_GRACE: Duration = Duration::from_millis(500);

    /// Capture and meter, without encoding or transmitting anything.
    ///
    /// What the setup screen's microphone test needs. It is `start` minus the network
    /// sender and minus the `current_player` requirement, and it deliberately reuses
    /// `listener` rather than growing a second capture path: the gate, the resampler,
    /// the processing core and the level emitter are then provably the same ones the
    /// real stream uses, so a meter that reads correctly here is evidence about the
    /// pipeline that will carry the user's voice.
    ///
    /// The consumer is dropped immediately, so `try_send` in the processing core fails
    /// from the first frame and the PCM is discarded where it is produced. Nothing
    /// queues and nothing is held.
    pub async fn start_metering(&mut self) -> Result<(), anyhow::Error> {
        self.shutdown.store(false, Ordering::Relaxed);
        // The one place the unquantised level is still published. See `listener`.
        self.raw_levels = true;

        let (producer, consumer) = flume::bounded::<AudioFrame>(1);
        drop(consumer);

        let capture_config = self.source.resolve_config(&self.device)?;

        let stream = self
            .listener(capture_config, producer, self.shutdown.clone())
            .inspect_err(|e| error!("input metering listener failed to start: {:?}", e))?;
        self.stream.hold(stream).await;

        // No jobs: metering is the capture callback and nothing else. `self.stream` is what says
        // this is running, which is why `is_stopped` reads both.
        self.jobs = JobSet::empty();
        Ok(())
    }

    /// Discard capture accounting, so the next stream is measured from zero.
    pub fn reset_stats(&self) {
        self.input_stats.reset();
    }

    /// Whether a session stream is supposed to be delivering frames right now.
    ///
    /// False for the setup screen's metering stream: it has no session to be rebuilt into, and
    /// `restart` would try to open the full network path for a client that has not connected.
    pub fn capture_expected(&self) -> bool {
        self.capture_expected.load(Ordering::Relaxed)
    }

    pub fn new(
        device: Option<AudioDevice>,
        source: AudioInputSource,
        bus: Arc<flume::Sender<NetworkPacket>>,
        metadata: Arc<moka::future::Cache<String, String>>,
        app_handle: tauri::AppHandle,
        recording_producer: Option<Arc<RecordingProducer>>,
        recording_active: Option<Arc<AtomicBool>>,
        recovery_tx: RecoverySender,
        input_stats: Arc<crate::diagnostics::InputPipelineStats>,
        levels: Arc<crate::audio::stream::level_bus::LevelBus>,
        #[cfg(feature = "bedrock-protocol")] player_state_cache: Option<
            Arc<BedrockPlayerStateCache>,
        >,
    ) -> Self {
        Self {
            device,
            source,
            bus,
            jobs: JobSet::empty(),
            stream: DeviceLease::empty(),
            capture_expected: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata,
            app_handle: app_handle.clone(),
            recording_producer,
            recording_active,
            recovery_tx,
            input_stats,
            levels,
            raw_levels: false,
            #[cfg(feature = "bedrock-protocol")]
            player_state_cache,
        }
    }

    // Builds the noise gate, resampler, and processing core from the resolved
    // capture config, then hands the core's push sink to the active source. The
    // source decides where frames originate — a live cpal callback or the test
    // bridge — while the gate/resampler/core wiring stays identical for both.
    // Returns the device handle for the caller to lease. Kept free of await points: the gate
    // settings are read under a `std::sync::Mutex`, whose guard is not `Send`.
    fn listener(
        &mut self,
        config: CaptureConfig,
        producer: flume::Sender<AudioFrame>,
        shutdown: Arc<AtomicBool>,
    ) -> Result<Option<rodio::cpal::Stream>, anyhow::Error> {
        let recovery_tx = self.recovery_tx.clone();
        let device = self.device.clone();

        let source = std::mem::replace(&mut self.source, AudioInputSource::Cpal);

        let sample_rate = config.sample_rate;
        let channels = config.channels;

        let settings = NOISE_GATE_SETTINGS.lock().unwrap();
        let noise_gate_settings =
            match serde_json::from_value::<NoiseGateSettings>(settings.clone()) {
                Ok(settings) => settings,
                Err(_) => NoiseGateSettings::default(),
            };
        drop(settings);

        let gate = NoiseGate::new(
            noise_gate_settings.open_threshold,
            noise_gate_settings.close_threshold,
            sample_rate as f32,
            match channels {
                1 => 1,
                2 => 2,
                _ => 2,
            },
            noise_gate_settings.release_rate,
            noise_gate_settings.attack_rate,
            noise_gate_settings.hold_time,
        );

        // Trailing frame count scales with release_rate (20ms per frame at 48kHz)
        let tail_frame_count: u32 = (noise_gate_settings.release_rate
            / OPUS_FRAME_DURATION_MS as f32)
            .ceil()
            .max(2.0) as u32;

        // Create resampler if the source sample rate is not 48 kHz
        let audio_resampler = match AudioResampler::new_if_needed(sample_rate) {
            Some(Ok(r)) => {
                warn!(
                    "Input device sample rate {} Hz requires resampling to 48 kHz. \
                     For optimal performance, use a device that supports 48 kHz natively.",
                    sample_rate
                );
                Some(r)
            }
            Some(Err(e)) => {
                curia::error!("failed to create audio resampler", {
                    defect: crate::logging::Defect::AudioDeviceRebuildFailed,
                    io: "input",
                    error: e.to_string(),
                });
                None
            }
            None => None,
        };

        // The raw level meter, for the setup screen only.
        //
        // Ten messages a second, which is affordable there and nowhere else: that screen has no
        // session behind it, so nothing else is competing for the webview, and the calibration
        // it exists for wants the real amplitude rather than a quantised one. In a session the
        // levels go to `LevelBus` instead, which is what keeps the message rate survivable on a
        // phone. Absent here, the task below never runs and the channel is never created.
        let (level_tx, level_rx) = flume::unbounded::<InputLevel>();
        let level_tx = self.raw_levels.then_some(level_tx);
        let level_handle = self.app_handle.clone();
        tokio::spawn(async move {
            use tauri::Manager;
            let mut batch_timer = tokio::time::interval(Duration::from_millis(100));
            let mut pending: Option<InputLevel> = None;

            loop {
                tokio::select! {
                    received = level_rx.recv_async() => {
                        match received {
                            Ok(level) => {
                                pending = Some(match pending {
                                    Some(prev) if prev.rms >= level.rms => InputLevel {
                                        rms: prev.rms,
                                        gate_open: prev.gate_open || level.gate_open,
                                    },
                                    _ => level,
                                });
                            }
                            // The core was dropped: the stream is gone and so is the meter.
                            Err(_) => break,
                        }
                    }
                    _ = batch_timer.tick() => {
                        if let Some(level) = pending.take() {
                            if let Some(broadcaster) = level_handle
                                .try_state::<crate::websocket::WebSocketBroadcaster>()
                            {
                                broadcaster.broadcast_input_level(level);
                            }
                        }
                    }
                }
            }
        });

        // The processing core owns the gate, resampler, buffers, and silence/tail
        // state. Its push sink is driven directly from the source so the real path
        // stays callback-driven with no intervening queue.
        let mut core = InputProcessCore::new(
            gate,
            audio_resampler,
            channels,
            sample_rate,
            tail_frame_count,
            producer,
            self.input_stats.clone(),
            level_tx,
            self.levels.clone(),
        );

        let process = move |data: &[f32]| {
            core.process(data);
        };

        let driver = source.drive(config, device, process, shutdown, recovery_tx)?;
        Ok(driver.stream)
    }

    fn sender(
        &mut self,
        consumer: flume::Receiver<AudioFrame>,
        shutdown: Arc<AtomicBool>,
        current_player_name: String,
        recording_active: Option<Arc<AtomicBool>>,
        source_sample_rate: u32,
        source_channels: u16,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        // Mobile audio backends (CoreAudio on iOS, AAudio on Android) should use
        // the default buffer size
        #[cfg(any(target_os = "ios", target_os = "android"))]
        let buffer_size = rodio::cpal::BufferSize::Default;

        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        let buffer_size = rodio::cpal::BufferSize::Fixed(crate::audio::BUFFER_SIZE);

        // Force 48 kHz if the source was not 48 kHz; the listener resamples to
        // match, so the Opus encoder and outgoing packets always run at 48 kHz.
        let effective_sample_rate = if source_sample_rate != crate::audio::AudioResampling::OPUS_SAMPLE_RATE {
            crate::audio::AudioResampling::OPUS_SAMPLE_RATE
        } else {
            source_sample_rate
        };

        let device_config = rodio::cpal::StreamConfig {
            channels: match source_channels {
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
            device_config.sample_rate.into(),
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
                curia::error!("could not create opus encoder", {
                    defect: crate::logging::Defect::EncoderInitFailed,
                    io: "input",
                    error: e.to_string(),
                });
                return Err(anyhow!("{}", e.to_string()));
            }
        };

        let bus = self.bus.clone();
        let recording_producer = self.recording_producer.clone();
        #[cfg(feature = "bedrock-protocol")]
        let player_state_cache = self.player_state_cache.clone();

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
                    warn!(
                        "Audio Input stream, quic sender received shutdown signal, and is now terminating."
                    );
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
                    let sample_to_process: Vec<f32> =
                        data_stream.drain(0..BUFFER_SIZE as usize).collect();

                    let encoded_data = match encoder
                        .encode_vec_float(&sample_to_process, sample_to_process.len() * 4)
                    {
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
                                    sample_rate: device_config.sample_rate,
                                    channels: device_config.channels.into(),
                                    emitter: RecordingPlayerData::for_input(
                                        current_player_name.clone(),
                                        // @todo: enrich with spatial data if available in the future
                                        // and gain setting
                                        None,
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
                            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                                encoded_data.clone(),
                                device_config.sample_rate,
                                None,
                                None,
                            )),
                                                    // Not a server fan-out, so this envelope carries no sequence.
                            ..Default::default()
                        },
                    };

                    if let Err(e) = tx.send_async(packet).await {
                        error!("Sending audio frame to Quic network thread failed: {:?}", e);
                    }
                }
            }
        });

        Ok(handle)
    }

    pub fn toggle(&self, event: StreamEvent) {
        match event {
            StreamEvent::Mute => {
                let current_state = MUTE_INPUT_STREAM.load(Ordering::Relaxed);
                MUTE_INPUT_STREAM.store(!current_state, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    pub fn mute_status(&self) -> bool {
        MUTE_INPUT_STREAM.load(Ordering::Relaxed)
    }
}
