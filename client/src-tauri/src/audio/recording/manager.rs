use super::{Recorder, RecordingData, RecordingProducer, RecordingConsumer};
use common::traits::StreamTrait;
use log::{error, info};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri_plugin_store::StoreExt;
use tokio::task::AbortHandle;

/// Central recording manager following NetworkStreamManager patterns
pub struct RecordingManager {
    recorder: Option<Recorder>,
    recording_state: Arc<AtomicBool>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    app_handle: tauri::AppHandle,

    // Recording channels (owned by manager)
    recording_producer: Arc<RecordingProducer>,
    recording_consumer: Arc<RecordingConsumer>,
}

impl RecordingManager {
    /// Create a new RecordingManager following NetworkStreamManager pattern
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        // Create internal recording channels
        let (recording_producer, recording_consumer) = flume::unbounded::<RecordingData>();

        Self {
            recorder: None,
            recording_state: Arc::new(AtomicBool::new(false)),
            jobs: Vec::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
            app_handle,
            recording_producer: Arc::new(recording_producer),
            recording_consumer: Arc::new(recording_consumer),
        }
    }

    /// Get the recording producer for streams to send data
    pub fn get_producer(&self) -> Arc<RecordingProducer> {
        self.recording_producer.clone()
    }

    /// Start a new recording session
    pub async fn start_recording(&mut self, current_player: String) -> Result<(), anyhow::Error> {
        if self.recording_state.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Recording already in progress"));
        }

        // Create new recorder instance with the consumer from app state
        let mut recorder = Recorder::new(
            current_player,
            self.app_handle.clone(),
            self.recording_consumer.clone(),
        ).await?;

        // Start the recorder
        recorder.start().await?;

        let session_id = recorder.session_id().to_string();
        self.recorder = Some(recorder);
        self.recording_state.store(true, Ordering::Relaxed);

        info!("Recording session {} started via RecordingManager", session_id);
        Ok(())
    }

    /// Stop the current recording session
    pub async fn stop_recording(&mut self) -> Result<(), anyhow::Error> {
        if !self.recording_state.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("No recording in progress"));
        }

        if let Some(recorder) = &mut self.recorder {
            recorder.stop().await?;
            info!("Recording session {} stopped via RecordingManager", recorder.session_id());
        }

        self.recorder = None;
        self.recording_state.store(false, Ordering::Relaxed);

        Ok(())
    }

    /// Check if recording is currently active
    pub fn is_recording(&self) -> bool {
        self.recording_state.load(Ordering::Relaxed)
    }

    /// Get current session ID if recording
    pub fn current_session_id(&self) -> Option<String> {
        self.recorder.as_ref().map(|r| r.session_id().to_string())
    }
}

impl RecordingManager {
    /// Extract current player from store
    async fn extract_current_player(&self) -> Option<String> {
        self.app_handle.store("store.json").ok()?
            .get("current_player")?
            .as_str()
            .map(String::from)
    }
}