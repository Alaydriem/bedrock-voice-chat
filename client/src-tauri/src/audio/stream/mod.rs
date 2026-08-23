mod activity_detector;
pub(crate) mod capture_availability;
pub(crate) mod capture_watchdog;
pub mod jitter_buffer;
pub(crate) mod level_bus;
pub(crate) mod rebuild_breaker;
pub mod stream_manager;

use crate::NetworkPacket;
use crate::audio::recording::RecordingManager;
use crate::audio::{AudioDevice, AudioDeviceType};
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::BedrockPlayerStateCache;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::JukeboxBeaconCache;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::JukeboxEjectInjector;
use anyhow::Error;
use common::structs::audio::StreamEvent;
use log::info;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::async_runtime::Mutex as TauriMutex;
use tauri_plugin_curia::curia;
use tokio::sync::mpsc;

use super::AudioPacket;
use capture_watchdog::{CaptureVerdict, CaptureWatchdog};
use rebuild_breaker::{RebuildBreaker, RebuildVerdict};
use stream_manager::{AudioInputSource, AudioOutputSink, StreamTrait, StreamTraitType};

pub(crate) use activity_detector::ActivityUpdate;

/// Event sent when a stream encounters an error requiring recovery
#[derive(Debug, Clone)]
pub enum StreamRecoveryEvent {
    DeviceError {
        device_type: AudioDeviceType,
        error: String,
    },
    /// Something changed that could make a device openable again. Clears an open breaker.
    Rearm { device_type: AudioDeviceType },
}

/// Sender type for recovery events (used by streams to signal errors)
pub type RecoverySender = mpsc::UnboundedSender<StreamRecoveryEvent>;

pub(crate) struct AudioStreamManager {
    producer: Arc<flume::Sender<NetworkPacket>>,
    consumer: Arc<flume::Receiver<AudioPacket>>,
    input: StreamTraitType,
    output: StreamTraitType,
    app_handle: tauri::AppHandle,
    recording_manager: Option<Arc<TauriMutex<RecordingManager>>>,
    recovery_tx: RecoverySender,
    recovery_rx: Option<mpsc::UnboundedReceiver<StreamRecoveryEvent>>,
    capture_watchdog_started: bool,
    // Every meter's state, and the only thing that publishes any of it to the webview.
    levels: Arc<level_bus::LevelBus>,
    level_publisher_started: bool,
    // Created here and shared with every stream this manager builds, so a device change or a
    // restart keeps writing into the same counters a diagnostic is already reading.
    input_stats: Arc<crate::diagnostics::InputPipelineStats>,
    // Written by the recovery monitor when it gives up on a device, read lock-free by the
    // runtime-state poll.
    capture_availability: Arc<capture_availability::CaptureAvailability>,
    // Owned here so it outlives the output streams that hand it their mixer. A rebuild
    // replaces the mixer; this instance is what carries the cue across that replacement.
    cue_sink: Arc<crate::audio::CueSink>,
    peer_registry: Arc<crate::diagnostics::PeerRegistry>,
    // Owned here for the same reason as `peer_registry`: the output stream writes into it while a
    // diagnostic reads it, and a stream restart must not swap the instance out from under the
    // reader.
    session_config: Arc<crate::diagnostics::SessionConfig>,
    #[cfg(feature = "bedrock-protocol")]
    player_state_cache: Option<Arc<BedrockPlayerStateCache>>,
    #[cfg(feature = "bedrock-protocol")]
    beacon_cache: Option<Arc<JukeboxBeaconCache>>,
    #[cfg(feature = "bedrock-protocol")]
    eject_injector: Option<Arc<JukeboxEjectInjector>>,
}

impl AudioStreamManager {
    // Handles for the diagnostics service. Both are created here and shared with every stream
    // this manager builds, so a device change or a stream restart keeps writing into the same
    // counters a reader already holds.
    pub fn input_stats(&self) -> Arc<crate::diagnostics::InputPipelineStats> {
        self.input_stats.clone()
    }

    pub fn peer_registry(&self) -> Arc<crate::diagnostics::PeerRegistry> {
        self.peer_registry.clone()
    }

    pub fn session_config(&self) -> Arc<crate::diagnostics::SessionConfig> {
        self.session_config.clone()
    }

    /// The meter bus, for the diagnostics service to report its published-message count.
    pub fn levels(&self) -> Arc<level_bus::LevelBus> {
        self.levels.clone()
    }

    /// Creates a new audio stream manager
    /// This is responsible for interfacing with all child threads
    pub fn new(
        producer: Arc<flume::Sender<NetworkPacket>>,
        consumer: Arc<flume::Receiver<AudioPacket>>,
        app_handle: tauri::AppHandle,
        recording_manager: Option<Arc<TauriMutex<RecordingManager>>>,
        #[cfg(feature = "bedrock-protocol")] player_state_cache: Option<
            Arc<BedrockPlayerStateCache>,
        >,
        #[cfg(feature = "bedrock-protocol")] beacon_cache: Option<Arc<JukeboxBeaconCache>>,
        #[cfg(feature = "bedrock-protocol")] eject_injector: Option<Arc<JukeboxEjectInjector>>,
    ) -> Self {
        Self::new_with_sources(
            producer,
            consumer,
            app_handle,
            recording_manager,
            AudioInputSource::Cpal,
            AudioOutputSink::Rodio,
            #[cfg(feature = "bedrock-protocol")]
            player_state_cache,
            #[cfg(feature = "bedrock-protocol")]
            beacon_cache,
            #[cfg(feature = "bedrock-protocol")]
            eject_injector,
        )
    }

    /// Creates a new audio stream manager with explicitly injected input
    /// source and output sink, used to swap in fake backends for tests while
    /// the production `new()` always selects the real Cpal/Rodio backends.
    /// The injected source/sink are consumed into the initial streams; device
    /// changes via `init`/`restart`/`reset` always rebuild on the real backend.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_sources(
        producer: Arc<flume::Sender<NetworkPacket>>,
        consumer: Arc<flume::Receiver<AudioPacket>>,
        app_handle: tauri::AppHandle,
        recording_manager: Option<Arc<TauriMutex<RecordingManager>>>,
        input_source: AudioInputSource,
        output_sink: AudioOutputSink,
        #[cfg(feature = "bedrock-protocol")] player_state_cache: Option<
            Arc<BedrockPlayerStateCache>,
        >,
        #[cfg(feature = "bedrock-protocol")] beacon_cache: Option<Arc<JukeboxBeaconCache>>,
        #[cfg(feature = "bedrock-protocol")] eject_injector: Option<Arc<JukeboxEjectInjector>>,
    ) -> Self {
        let (recovery_tx, recovery_rx) = mpsc::unbounded_channel::<StreamRecoveryEvent>();
        let input_stats = Arc::new(crate::diagnostics::InputPipelineStats::new());
        let capture_availability = capture_availability::CaptureAvailability::new_shared();
        let cue_sink = crate::audio::CueSink::new_shared();
        let peer_registry = crate::diagnostics::PeerRegistry::new_shared();
        let session_config = Arc::new(crate::diagnostics::SessionConfig::new());
        let levels = level_bus::LevelBus::new_shared();

        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                None,
                input_source,
                producer.clone(),
                Arc::new(moka::future::Cache::builder().build()),
                app_handle.clone(),
                None,
                None,
                recovery_tx.clone(),
                input_stats.clone(),
                levels.clone(),
                #[cfg(feature = "bedrock-protocol")]
                player_state_cache.clone(),
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                None,
                output_sink,
                consumer.clone(),
                Arc::new(moka::future::Cache::builder().build()),
                app_handle.clone(),
                None,
                None,
                recovery_tx.clone(),
                peer_registry.clone(),
                levels.clone(),
                session_config.clone(),
                #[cfg(feature = "bedrock-protocol")]
                beacon_cache.clone(),
                #[cfg(feature = "bedrock-protocol")]
                eject_injector.clone(),
            )),
            app_handle: app_handle.clone(),
            recording_manager,
            recovery_tx,
            recovery_rx: Some(recovery_rx),
            capture_watchdog_started: false,
            levels: levels.clone(),
            level_publisher_started: false,
            input_stats,
            capture_availability,
            cue_sink,
            peer_registry,
            session_config,
            #[cfg(feature = "bedrock-protocol")]
            player_state_cache,
            #[cfg(feature = "bedrock-protocol")]
            beacon_cache,
            #[cfg(feature = "bedrock-protocol")]
            eject_injector,
        }
    }

    /// Spawns the recovery monitor task if not already spawned.
    /// Must be called from an async context.
    ///
    /// The rebuild happens here rather than in the webview. Recovery that depends on a live
    /// webview is unavailable in exactly the conditions that need it, and while both did it a
    /// single device error rebuilt the stream twice.
    fn spawn_recovery_monitor(&mut self) {
        if let Some(mut recovery_rx) = self.recovery_rx.take() {
            let app_handle = self.app_handle.clone();
            let availability = self.capture_availability.clone();
            tokio::spawn(async move {
                let mut breaker = RebuildBreaker::new();

                while let Some(event) = recovery_rx.recv().await {
                    match event {
                        StreamRecoveryEvent::DeviceError { device_type, error } => {
                            Self::recover(
                                &app_handle,
                                &availability,
                                &mut breaker,
                                device_type,
                                error,
                            )
                            .await;
                        }
                        StreamRecoveryEvent::Rearm { device_type } => {
                            breaker.rearm(&device_type);
                            if matches!(device_type, AudioDeviceType::InputDevice) {
                                availability.set(true);
                            }
                        }
                    }
                }
            });
        }
    }

    /// One failed stream, taken as far as the breaker allows.
    ///
    /// Attempts log at `WARN`, which reaches Sentry as a breadcrumb and not as an event. The one
    /// `ERROR` is the breaker opening, which is the fact worth paying for: this device will not
    /// open, rather than it failed once more.
    ///
    /// A rebuild that fails is the next iteration rather than a fresh event: `restart` can fail
    /// before the device is opened, and nothing then reaches the capture callback that would
    /// have sent one through the channel.
    async fn recover(
        app_handle: &tauri::AppHandle,
        availability: &Arc<capture_availability::CaptureAvailability>,
        breaker: &mut RebuildBreaker,
        device_type: AudioDeviceType,
        error: String,
    ) {
        let is_input = matches!(device_type, AudioDeviceType::InputDevice);

        let _ = app_handle.emit(
            "audio-stream-recovery",
            serde_json::json!({
                "device_type": match device_type {
                    AudioDeviceType::InputDevice => "InputDevice",
                    AudioDeviceType::OutputDevice => "OutputDevice",
                },
                "error": error,
            }),
        );

        let mut reason = error;

        loop {
            let (after, attempt) = match breaker.observe_failure(&device_type) {
                RebuildVerdict::Retry { after, attempt } => (after, attempt),
                RebuildVerdict::Open => {
                    curia::error!("device could not be opened and will not be retried", {
                        defect: crate::logging::Defect::AudioDeviceRebuildFailed,
                        io: if is_input { "input" } else { "output" },
                        attempts: RebuildBreaker::MAX_ATTEMPTS,
                        error: reason.to_string(),
                    });
                    if is_input {
                        availability.set(false);
                    }
                    break;
                }
            };

            log::warn!(
                "{:?} stream failed ({}); rebuild {} of {} in {:?}",
                device_type,
                reason,
                attempt,
                RebuildBreaker::MAX_ATTEMPTS,
                after
            );
            tokio::time::sleep(after).await;

            // The manager is gone, which means the application is shutting down. Nothing left
            // to rebuild into.
            let Some(state) = app_handle.try_state::<TauriMutex<AudioStreamManager>>() else {
                break;
            };

            let outcome = {
                let mut asm = state.lock().await;
                asm.restart(device_type.clone()).await
            };

            match outcome {
                Ok(()) => {
                    breaker.observe_success(&device_type);
                    if is_input {
                        availability.set(true);
                    }
                    info!("{:?} stream rebuilt", device_type);
                    break;
                }
                Err(e) => reason = format!("{:?}", e),
            }
        }
    }

    /// Clears an open breaker, so a device the user just changed is tried again.
    pub fn rearm_rebuilds(&self, device: AudioDeviceType) {
        let _ = self.recovery_tx.send(StreamRecoveryEvent::Rearm {
            device_type: device,
        });
    }

    /// Whether the capture device could be opened. False only after every attempt was spent.
    pub fn capture_availability(&self) -> Arc<capture_availability::CaptureAvailability> {
        self.capture_availability.clone()
    }

    /// Where mute and deafen cues are played.
    pub fn cue_sink(&self) -> Arc<crate::audio::CueSink> {
        self.cue_sink.clone()
    }

    /// How often the watchdog reads the capture counter.
    const CAPTURE_POLL: std::time::Duration = std::time::Duration::from_secs(1);

    /// Consecutive reads with no new frames before the capture stream is rebuilt.
    ///
    /// A healthy stream advances the counter about fifty times per read, so three empty reads
    /// are far outside any scheduling hiccup, and three seconds of silence is short enough that
    /// the other side hears a gap rather than a person who left.
    const CAPTURE_DEAD_AFTER: u32 = 3;

    /// Rebuilds the capture stream when the device stops delivering without saying so.
    ///
    /// The error callback is the only other thing that notices a dead microphone, and it fires
    /// on `StreamError` alone. A capture stream has more ways to stop than to fail — an endpoint
    /// that disappears quietly, an audio focus a phone hands to another application, a callback
    /// that stops being scheduled — and none of those raise one. The stream then stays running,
    /// `is_stopped` stays false because its task handles are still alive, and the microphone is
    /// dead until the application is restarted.
    ///
    /// Restarts here rather than by emitting to the frontend, which is what the error path does:
    /// recovery that depends on a live webview is unavailable in exactly the conditions that
    /// need it most, and routing both through the same event would restart the stream twice.
    fn spawn_capture_watchdog(&mut self) {
        if self.capture_watchdog_started {
            return;
        }
        self.capture_watchdog_started = true;

        let app_handle = self.app_handle.clone();
        tokio::spawn(async move {
            let mut watchdog = CaptureWatchdog::new(Self::CAPTURE_DEAD_AFTER);
            let mut ticker = tokio::time::interval(Self::CAPTURE_POLL);
            // Without this a suspended process wakes to a burst of backdated ticks, every one of
            // them reading the same counter, and declares a device dead that was merely asleep.
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                ticker.tick().await;

                let Some(state) = app_handle.try_state::<TauriMutex<AudioStreamManager>>() else {
                    continue;
                };

                let mut asm = state.lock().await;
                let verdict = watchdog.observe(
                    asm.input.capture_expected(),
                    asm.input_stats.frames_captured(),
                );

                if verdict != CaptureVerdict::Dead {
                    continue;
                }

                curia::warn!("capture stream delivered no frames and reported no error; rebuilding", {
                    io: "input",
                    quiet_secs: Self::CAPTURE_DEAD_AFTER as u64 * Self::CAPTURE_POLL.as_secs(),
                });
                if let Err(e) = asm.restart(AudioDeviceType::InputDevice).await {
                    curia::error!("capture watchdog could not rebuild the input stream", {
                        defect: crate::logging::Defect::AudioDeviceRebuildFailed,
                        io: "input",
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// How often the publisher looks at the bus. Not how often it sends.
    ///
    /// Sampling is free — the bus is two atomics and a map — so this is set by how quickly a
    /// press-to-talk should light the meter rather than by what the webview can carry. What
    /// reaches the webview is decided by `LevelEmitPolicy`.
    const LEVEL_POLL: std::time::Duration = std::time::Duration::from_millis(100);

    /// The only thing that publishes meter levels to the webview.
    ///
    /// There were two, each on a fixed 100 ms timer: the capture path's raw level and
    /// the mixer's `audio-activity`. Twenty messages a second between them, whether or not
    /// anything had changed. On Android every one of those is a unit of main-thread work —
    /// dequeue, marshal a JavaScript string over JNI, evaluate it — on the same thread that
    /// lays out and paints the meters they feed, so the meter was the first thing to starve
    /// exactly when the most was happening.
    ///
    /// One emitter, sending on change instead of on a clock.
    fn spawn_level_publisher(&mut self) {
        if self.level_publisher_started {
            return;
        }
        self.level_publisher_started = true;

        let app_handle = self.app_handle.clone();
        let levels = self.levels.clone();
        tokio::spawn(async move {
            let mut policy = level_bus::LevelEmitPolicy::new();
            let mut ticker = tokio::time::interval(Self::LEVEL_POLL);
            // A suspended process otherwise wakes to a burst of backdated ticks and publishes
            // several times over for one state, which is the opposite of the point.
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                ticker.tick().await;

                let snapshot = levels.snapshot();
                if !policy.admit(std::time::Instant::now(), &snapshot) {
                    continue;
                }

                match app_handle.try_state::<crate::websocket::WebSocketBroadcaster>() {
                    Some(broadcaster) => {
                        broadcaster.broadcast_levels(snapshot);
                        levels.record_emitted();
                    }
                    None => log::warn!("no push channel to publish audio levels on"),
                }
            }
        });
    }

    /// Initializes a given input or output stream with a specific device, then starts it
    pub async fn init(&mut self, device: AudioDevice) {
        // Spawn recovery monitor on first init (now we're in async context)
        self.spawn_recovery_monitor();
        self.spawn_capture_watchdog();
        self.spawn_level_publisher();

        // Stop the current stream if we're re-initializing a new one so we don't
        // have dangling thread pointers
        _ = self.stop(device.clone().io).await;

        // Get recording producer and flag from manager if available
        let (recording_producer, recording_flag) = if let Some(ref rm) = self.recording_manager {
            let manager = rm.lock().await;
            (
                Some(manager.get_producer()),
                Some(manager.get_recording_flag()),
            )
        } else {
            (None, None)
        };

        match device.io {
            AudioDeviceType::InputDevice => {
                // A rebuilt input stream is a fresh measurement. This is also what
                // clears the setup screen's metering totals: that stream captures
                // frames and sends none, and a full page navigation into the dashboard
                // never gives the frontend a chance to stop it, so the reset has to sit
                // on the path into the session rather than on the way out of setup.
                self.input.reset_stats();
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    Some(device),
                    stream_manager::AudioInputSource::Cpal,
                    self.producer.clone(),
                    self.input.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer.clone(),
                    recording_flag.clone(),
                    self.recovery_tx.clone(),
                    self.input_stats.clone(),
                    self.levels.clone(),
                    #[cfg(feature = "bedrock-protocol")]
                    self.player_state_cache.clone(),
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    Some(device),
                    stream_manager::AudioOutputSink::Rodio,
                    self.consumer.clone(),
                    self.output.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer,
                    recording_flag,
                    self.recovery_tx.clone(),
                    self.peer_registry.clone(),
                    self.levels.clone(),
                    self.session_config.clone(),
                    #[cfg(feature = "bedrock-protocol")]
                    self.beacon_cache.clone(),
                    #[cfg(feature = "bedrock-protocol")]
                    self.eject_injector.clone(),
                ));
            }
        }
    }

    /// Restarts the audio stream for a given device
    /// This will stop the stream, create a new StreamManager with the same underlying device
    /// Then start a new stream in its place
    #[allow(unused)]
    #[tracing::instrument(skip(self), fields(device = ?device))]
    pub async fn restart(&mut self, device: AudioDeviceType) -> Result<(), Error> {
        // Stop the audio stream
        _ = self.stop(device.clone()).await;

        // Get recording producer and flag from manager if available
        let (recording_producer, recording_flag) = if let Some(ref rm) = self.recording_manager {
            let manager = rm.lock().await;
            (
                Some(manager.get_producer()),
                Some(manager.get_recording_flag()),
            )
        } else {
            (None, None)
        };

        match device {
            AudioDeviceType::InputDevice => {
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    self.input.get_device(),
                    stream_manager::AudioInputSource::Cpal,
                    self.producer.clone(),
                    self.input.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer.clone(),
                    recording_flag.clone(),
                    self.recovery_tx.clone(),
                    self.input_stats.clone(),
                    self.levels.clone(),
                    #[cfg(feature = "bedrock-protocol")]
                    self.player_state_cache.clone(),
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    self.output.get_device(),
                    stream_manager::AudioOutputSink::Rodio,
                    self.consumer.clone(),
                    self.output.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer,
                    recording_flag,
                    self.recovery_tx.clone(),
                    self.peer_registry.clone(),
                    self.levels.clone(),
                    self.session_config.clone(),
                    #[cfg(feature = "bedrock-protocol")]
                    self.beacon_cache.clone(),
                    #[cfg(feature = "bedrock-protocol")]
                    self.eject_injector.clone(),
                ));
            }
        };

        self.start(device).await
    }

    /// Starts the stream for a given audio device type
    pub async fn start(&mut self, device: AudioDeviceType) -> Result<(), Error> {
        // Start the new device
        match device {
            AudioDeviceType::InputDevice => match self.input.is_stopped() {
                true => self.input.start().await,
                false => Err(anyhow::anyhow!(format!(
                    "{} audio stream is already running!",
                    device.store_key()
                ))),
            },
            AudioDeviceType::OutputDevice => match self.output.is_stopped() {
                true => self.output.start().await,
                false => Err(anyhow::anyhow!(format!(
                    "{} audio stream is already running!",
                    device.store_key()
                ))),
            },
        }
    }

    /// Stops the audio stream for the given device
    /// This permanently shuts down all associated threads
    /// To restart the device, either call restart(), or re-initialize the device
    pub async fn stop(&mut self, device: AudioDeviceType) -> Result<(), Error> {
        match device {
            AudioDeviceType::InputDevice => self.input.stop().await?,
            AudioDeviceType::OutputDevice => self.output.stop().await?,
        };

        Ok(())
    }

    /// Start capturing purely to drive a level meter, before there is a session.
    ///
    /// Used by the setup screen's microphone test. The device has to be passed in: the
    /// manager builds its input stream with no device and only learns one from `init`,
    /// which nothing has called this early — the dashboard is what normally calls it, and
    /// setup runs before the dashboard. Without this the capture config cannot resolve and
    /// the meter reads flat.
    ///
    /// Safe to leave running into a navigation: `init` stops the input stream before it
    /// replaces it, so the session's own stream cannot end up stacked on top of this one
    /// even when the page tears down without the frontend getting a chance to stop it.
    pub async fn start_input_metering(&mut self, device: AudioDevice) -> Result<(), Error> {
        self.init(device).await;
        self.input.start_metering().await
    }

    /// Stop the metering stream and discard what it counted, so the session's own
    /// capture is measured from zero.
    pub async fn stop_input_metering(&mut self) -> Result<(), Error> {
        self.input.stop().await?;
        self.input.reset_stats();
        Ok(())
    }

    pub async fn is_stopped(&mut self, device: &AudioDeviceType) -> Result<bool, Error> {
        let status = match device {
            AudioDeviceType::InputDevice => self.input.is_stopped(),
            AudioDeviceType::OutputDevice => self.output.is_stopped(),
        };

        Ok(status)
    }

    pub async fn metadata(
        &mut self,
        key: String,
        value: String,
        device: &AudioDeviceType,
    ) -> Result<(), Error> {
        // Both the dashboard UI and the ControlActionsManager feed per-player gain
        // changes through this key; nudge the control-plane reporter so the
        // server's preference cache mirrors the persisted store.
        if key == "player_gain_store" {
            if let Some(bus) = self.app_handle.try_state::<crate::control::ControlStateBus>() {
                bus.preferences();
            }
        }
        match device {
            AudioDeviceType::InputDevice => self.input.metadata(key, value).await,
            AudioDeviceType::OutputDevice => self.output.metadata(key, value).await,
        }
    }

    /// The current value of a metadata key on one stream, or `None` when unset.
    ///
    /// The metadata entry rather than whatever atomic it seeds: this is the copy a rebuilt stream
    /// restores from, so it is the one a read-then-write has to consult to stay in step with what
    /// a rebuild would put back.
    pub async fn metadata_value(&self, key: &str, device: &AudioDeviceType) -> Option<String> {
        let metadata = match device {
            AudioDeviceType::InputDevice => self.input.get_metadata(),
            AudioDeviceType::OutputDevice => self.output.get_metadata(),
        };

        metadata.get(key).await
    }

    pub async fn toggle(
        &mut self,
        device: &AudioDeviceType,
        event: StreamEvent,
    ) -> Result<(), Error> {
        match device {
            AudioDeviceType::InputDevice => self.input.toggle(event),
            AudioDeviceType::OutputDevice => self.output.toggle(event),
        };

        Ok(())
    }

    pub async fn mute_status(&mut self, device: &AudioDeviceType) -> Result<bool, Error> {
        let status = match device {
            AudioDeviceType::InputDevice => self.input.mute_status(),
            AudioDeviceType::OutputDevice => self.output.mute_status(),
        };

        Ok(status)
    }

    /// Discards whatever the network has queued for the output device, returning how much went.
    ///
    /// The channel is created once at startup and every stream built here holds an `Arc` clone,
    /// so replacing a stream leaves the queue as it was. At 20 ms a frame, its 10000 slots are
    /// over three minutes of audio that would otherwise play at once.
    pub fn drain_inbound(&self) -> usize {
        self.consumer.drain().count()
    }

    /// Stops both sides in the order that leaves nothing queued behind them, then rebuilds.
    ///
    /// Each queue is drained only after the stream that fills it has stopped: the outbound queue
    /// is fed by this manager's input stream, the inbound one by the network manager's. Draining
    /// ahead of that clears nothing durable.
    ///
    /// Locks the network manager while holding this one. Nothing else takes both; if something
    /// comes to, it must take them in this order.
    pub async fn restart_session(&mut self) -> Result<(), Error> {
        self.input.stop().await?;

        let discarded_outbound = match self
            .app_handle
            .try_state::<TauriMutex<crate::network::NetworkStreamManager>>()
        {
            Some(nsm) => {
                let mut nsm = nsm.lock().await;
                nsm.reset().await?;
                nsm.drain_outbound()
            }
            None => 0,
        };

        let discarded_inbound = self.drain_inbound();

        self.output.stop().await?;

        if discarded_outbound > 0 || discarded_inbound > 0 {
            log::info!(
                "Restart discarded {} outbound and {} inbound queued frames",
                discarded_outbound,
                discarded_inbound
            );
        }

        self.rebuild_streams().await
    }

    /// Resets the audio stream manager by stopping all streams and recreating them
    /// This is used when a full reset is needed (e.g., after page refresh)
    pub async fn reset(&mut self) -> Result<(), Error> {
        // Stop both streams concurrently
        let (_, _) = tokio::join!(self.input.stop(), self.output.stop());

        self.rebuild_streams().await
    }

    async fn rebuild_streams(&mut self) -> Result<(), Error> {
        // Get recording producer and flag from manager if available
        let (recording_producer, recording_flag) = if let Some(ref rm) = self.recording_manager {
            let manager = rm.lock().await;
            (
                Some(manager.get_producer()),
                Some(manager.get_recording_flag()),
            )
        } else {
            (None, None)
        };

        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            None,
            stream_manager::AudioInputSource::Cpal,
            self.producer.clone(),
            self.input.get_metadata().clone(),
            self.app_handle.clone(),
            recording_producer.clone(),
            recording_flag.clone(),
            self.recovery_tx.clone(),
            self.input_stats.clone(),
            self.levels.clone(),
            #[cfg(feature = "bedrock-protocol")]
            self.player_state_cache.clone(),
        ));

        // Recreate output stream, preserving metadata
        self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
            None,
            stream_manager::AudioOutputSink::Rodio,
            self.consumer.clone(),
            self.output.get_metadata().clone(),
            self.app_handle.clone(),
            recording_producer,
            recording_flag,
            self.recovery_tx.clone(),
            self.peer_registry.clone(),
            self.levels.clone(),
            self.session_config.clone(),
            #[cfg(feature = "bedrock-protocol")]
            self.beacon_cache.clone(),
            #[cfg(feature = "bedrock-protocol")]
            self.eject_injector.clone(),
        ));

        Ok(())
    }

    /// Returns the currently tracked players with their game type
    pub fn get_current_players(&self) -> std::collections::HashMap<String, Option<String>> {
        match &self.output {
            StreamTraitType::Output(stream) => stream.get_current_players(),
            _ => std::collections::HashMap::new(),
        }
    }
}
