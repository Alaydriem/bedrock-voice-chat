mod activity_detector;
pub(crate) mod capture_watchdog;
pub mod jitter_buffer;
pub(crate) mod level_bus;
pub(crate) mod stream_manager;

use crate::NetworkPacket;
use crate::audio::recording::RecordingManager;
use crate::audio::types::{AudioDevice, AudioDeviceType};
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::AnnounceInjector;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::BedrockPlayerStateCache;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::JukeboxBeaconCache;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::JukeboxEjectInjector;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::PresenceInjector;
use anyhow::Error;
use common::structs::audio::StreamEvent;
use log::info;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::async_runtime::Mutex as TauriMutex;
use tokio::sync::mpsc;

use super::AudioPacket;
use capture_watchdog::{CaptureVerdict, CaptureWatchdog};
use stream_manager::{AudioInputSource, AudioOutputSink, StreamTrait, StreamTraitType};

pub(crate) use activity_detector::ActivityUpdate;

/// Event sent when a stream encounters an error requiring recovery
#[derive(Debug, Clone)]
pub enum StreamRecoveryEvent {
    DeviceError {
        device_type: AudioDeviceType,
        error: String,
    },
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
    #[cfg(feature = "bedrock-protocol")]
    presence_injector: Option<Arc<PresenceInjector>>,
    #[cfg(feature = "bedrock-protocol")]
    announce_injector: Option<Arc<AnnounceInjector>>,
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

    /// Whether the session's own capture stream is running right now.
    ///
    /// Asked rather than inferred. The settings meter used to decide this by waiting to see
    /// whether level events arrived, which stopped being sound the moment levels were only
    /// published on change: a quiet room and a dead capture then look identical, and guessing
    /// wrong costs a live stream — `start_input_metering` runs `init`, which tears the session
    /// capture down and rebuilds it with no network sender attached.
    pub fn input_capture_active(&self) -> bool {
        self.input.capture_expected()
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
        #[cfg(feature = "bedrock-protocol")] presence_injector: Option<Arc<PresenceInjector>>,
        #[cfg(feature = "bedrock-protocol")] announce_injector: Option<Arc<AnnounceInjector>>,
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
            #[cfg(feature = "bedrock-protocol")]
            presence_injector,
            #[cfg(feature = "bedrock-protocol")]
            announce_injector,
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
        #[cfg(feature = "bedrock-protocol")] presence_injector: Option<Arc<PresenceInjector>>,
        #[cfg(feature = "bedrock-protocol")] announce_injector: Option<Arc<AnnounceInjector>>,
    ) -> Self {
        let (recovery_tx, recovery_rx) = mpsc::unbounded_channel::<StreamRecoveryEvent>();
        let input_stats = Arc::new(crate::diagnostics::InputPipelineStats::new());
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
                #[cfg(feature = "bedrock-protocol")]
                presence_injector.clone(),
                #[cfg(feature = "bedrock-protocol")]
                announce_injector.clone(),
            )),
            app_handle: app_handle.clone(),
            recording_manager,
            recovery_tx,
            recovery_rx: Some(recovery_rx),
            capture_watchdog_started: false,
            levels: levels.clone(),
            level_publisher_started: false,
            input_stats,
            peer_registry,
            session_config,
            #[cfg(feature = "bedrock-protocol")]
            player_state_cache,
            #[cfg(feature = "bedrock-protocol")]
            beacon_cache,
            #[cfg(feature = "bedrock-protocol")]
            eject_injector,
            #[cfg(feature = "bedrock-protocol")]
            presence_injector,
            #[cfg(feature = "bedrock-protocol")]
            announce_injector,
        }
    }

    /// Spawns the recovery monitor task if not already spawned.
    /// Must be called from an async context.
    fn spawn_recovery_monitor(&mut self) {
        if let Some(mut recovery_rx) = self.recovery_rx.take() {
            let app_handle = self.app_handle.clone();
            tokio::spawn(async move {
                while let Some(event) = recovery_rx.recv().await {
                    match event {
                        StreamRecoveryEvent::DeviceError { device_type, error } => {
                            info!("Stream recovery triggered for {:?}: {}", device_type, error);
                            // Emit event for frontend to handle recovery
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
                        }
                    }
                }
            });
        }
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

                log::warn!(
                    "Capture stream delivered no frames for {}s and reported no error; rebuilding it",
                    Self::CAPTURE_DEAD_AFTER as u64 * Self::CAPTURE_POLL.as_secs()
                );
                if let Err(e) = asm.restart(AudioDeviceType::InputDevice).await {
                    log::error!("Capture watchdog could not rebuild the input stream: {:?}", e);
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
    /// There were two, each on a fixed 100 ms timer: the capture path's `audio-input-level` and
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

                match app_handle.emit(crate::events::event::AUDIO_LEVELS, &snapshot) {
                    Ok(()) => levels.record_emitted(),
                    Err(e) => log::warn!("Failed to emit audio levels: {}", e),
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
                    #[cfg(feature = "bedrock-protocol")]
                    self.presence_injector.clone(),
                    #[cfg(feature = "bedrock-protocol")]
                    self.announce_injector.clone(),
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
                    #[cfg(feature = "bedrock-protocol")]
                    self.presence_injector.clone(),
                    #[cfg(feature = "bedrock-protocol")]
                    self.announce_injector.clone(),
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
            #[cfg(feature = "bedrock-protocol")]
            self.presence_injector.clone(),
            #[cfg(feature = "bedrock-protocol")]
            self.announce_injector.clone(),
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
