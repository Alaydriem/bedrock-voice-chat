use super::{RawRecordingData, Recorder, RecordingConsumer, RecordingProducer};
use common::traits::StreamTrait;
use log::info;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::Emitter;

/// Central recording manager following NetworkStreamManager patterns
pub struct RecordingManager {
    recorder: Option<Recorder>,
    recording_state: Arc<AtomicBool>,
    app_handle: tauri::AppHandle,

    // Whether the connected server permits recording. Permissive until a connection
    // says otherwise, so a client that has not connected yet behaves as it always did.
    allowed: bool,

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

        // Emit event to notify UI components
        self.app_handle.emit("recording:started", &session_id).ok();

        info!(
            "Recording session {} started via RecordingManager",
            session_id
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

        if let Some(recorder) = &mut self.recorder {
            // Now drain and finish
            recorder.stop().await?;
            info!(
                "Recording session {} stopped via RecordingManager",
                recorder.session_id()
            );
        }

        self.recorder = None;

        // Emit event to notify UI components
        self.app_handle.emit("recording:stopped", ()).ok();

        Ok(())
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
