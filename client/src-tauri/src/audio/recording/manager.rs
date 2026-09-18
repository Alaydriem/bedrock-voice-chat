use super::{RawRecordingData, Recorder, RecordingConsumer, RecordingProducer};
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use common::traits::StreamTrait;
use log::info;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use tauri::{Emitter, Manager};

/// Central recording manager following NetworkStreamManager patterns
pub struct RecordingManager {
    recorder: Option<Recorder>,
    recording_state: Arc<AtomicBool>,
    app_handle: tauri::AppHandle,

    // Whether the connected server permits recording. Permissive until a connection
    // says otherwise, so a client that has not connected yet behaves as it always did.
    allowed: bool,

    // When the live session was armed. Taken on stop, so a second recording is timed from
    // its own start rather than from the first one.
    started_at: Option<Instant>,

    // Recording channels (owned by manager)
    recording_producer: Arc<RecordingProducer>,
    recording_consumer: Arc<RecordingConsumer>,
}

impl RecordingManager {
    /// Create a new RecordingManager following NetworkStreamManager pattern
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        // Create internal recording channels
        let (recording_producer, recording_consumer) = flume::unbounded::<RawRecordingData>();

        Self {
            recorder: None,
            recording_state: Arc::new(AtomicBool::new(false)),
            app_handle,
            allowed: true,
            started_at: None,
            recording_producer: Arc::new(recording_producer),
            recording_consumer: Arc::new(recording_consumer),
        }
    }

    /// Get the recording producer for streams to send data
    pub fn get_producer(&self) -> Arc<RecordingProducer> {
        self.recording_producer.clone()
    }

    /// Returns the recording active flag for streams to check
    /// Streams should check this flag before sending recording data
    pub fn get_recording_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.recording_state)
    }

    /// Adopt the connected server's recording policy.
    pub fn set_allowed(&mut self, allowed: bool) {
        self.allowed = allowed;
    }

    /// Whether the connected server permits arming a recording.
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// Start a new recording session
    ///
    /// Every surface that arms a recording — the record button, the global hotkey, the
    /// Stream Deck socket and the in-game panel — reaches this method, which is why the
    /// operator's policy is checked here rather than at any one of them.
    pub async fn start_recording(&mut self, current_player: String) -> Result<(), anyhow::Error> {
        if !self.allowed {
            return Err(anyhow::anyhow!(
                "RECORDING_DISABLED: this server does not permit recording"
            ));
        }
        if self.recording_state.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Recording already in progress"));
        }

        // Create new recorder instance with the consumer from app state
        let mut recorder = Recorder::new(
            current_player,
            self.app_handle.clone(),
            self.recording_consumer.clone(),
        )
        .await?;

        // Start the recorder
        recorder.start().await?;

        let session_id = recorder.session_id().to_string();
        self.recorder = Some(recorder);
        // Set flag AFTER recorder starts - streams can now send data
        self.recording_state.store(true, Ordering::SeqCst);
        self.started_at = Some(Instant::now());

        // Emit event to notify UI components
        self.app_handle.emit("recording:started", &session_id).ok();

        self.track(
            AnalyticsEvent::RecordingStarted,
            AnalyticsEventData::new().insert("session_id", session_id),
        );

        Ok(())
    }

    /// Stop the current recording session
    pub async fn stop_recording(&mut self) -> Result<(), anyhow::Error> {
        if !self.recording_state.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("No recording in progress"));
        }

        // Set flag FIRST so streams stop sending new data immediately
        self.recording_state.store(false, Ordering::SeqCst);

        // Read before the recorder is dropped, which is the only thing that knows the id.
        let session_id = self.current_session_id();
        let duration_ms = self
            .started_at
            .take()
            .map(|started| started.elapsed().as_millis() as u64);

        if let Some(recorder) = &mut self.recorder {
            recorder.stop().await?;
        }

        self.recorder = None;

        // Emit event to notify UI components
        self.app_handle.emit("recording:stopped", ()).ok();

        // Measured to the stop rather than to the rendered file: what this answers is how long
        // somebody left it running, which separates an abandoned arm from a real session.
        let mut data = AnalyticsEventData::new();
        if let Some(session_id) = session_id {
            data = data.insert("session_id", session_id);
        }
        if let Some(duration_ms) = duration_ms {
            data = data.insert("duration_ms", duration_ms);
        }
        self.track(AnalyticsEvent::RecordingStopped, data);

        Ok(())
    }

    /// Reports a recording event, when analytics are running.
    ///
    /// Placed on the manager rather than on the Tauri commands because every surface that
    /// arms a recording — the record button, the global hotkey, the Stream Deck socket and
    /// the in-game panel — reaches the manager, and only two of them reach a command.
    fn track(&self, event: AnalyticsEvent, data: AnalyticsEventData) {
        if let Some(analytics) = self
            .app_handle
            .try_state::<Arc<crate::analytics::AnalyticsService>>()
        {
            analytics.track(event, Some(data));
        }
    }

    /// Check if recording is currently active
    pub fn is_recording(&self) -> bool {
        self.recording_state.load(Ordering::SeqCst)
    }

    /// Get current session ID if recording
    pub fn current_session_id(&self) -> Option<String> {
        self.recorder.as_ref().map(|r| r.session_id().to_string())
    }
}
