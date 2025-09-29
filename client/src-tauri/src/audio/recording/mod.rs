mod manager;

use common::structs::packet::PacketOwner;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
use tokio::task::AbortHandle;
use uuid::{NoContext, Timestamp, Uuid};

pub use manager::RecordingManager;

/// Type aliases for recording channels (following audio/network pattern)
pub type RecordingProducer = flume::Sender<RecordingData>;
pub type RecordingConsumer = flume::Receiver<RecordingData>;

/// Data types that can be recorded
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RecordingData {
    /// Input data from microphone (raw PCM + encoded opus)
    InputData {
        timestamp_ms: u64,
        sample_rate: u32,
        pcm_data: Vec<f32>,
        opus_data: Vec<u8>,
    },
    /// Output data from remote players (encoded opus frames)
    OutputData {
        timestamp_ms: u64,
        sample_rate: u32,
        opus_data: Vec<u8>,
        owner: Option<PacketOwner>,
        coordinate: Option<common::Coordinate>,
        orientation: Option<common::Orientation>,
        dimension: Option<common::Dimension>,
        is_spatial: bool,
    },
}

/// Session manifest for reconstruction
#[derive(Serialize, Deserialize, Clone)]
pub struct SessionManifest {
    pub session_id: String,
    pub start_timestamp: u64,
    pub end_timestamp: Option<u64>,
    pub duration_ms: Option<u64>,
    pub emitter_player: String,
    pub participants: Vec<String>,
    pub sample_rate: u32,
    pub created_at: String,
}

/// Core recorder that handles WAL storage and session management
pub struct Recorder {
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    session_id: String,
    manifest: SessionManifest,
    recording_path: PathBuf,
    recording_consumer: Arc<RecordingConsumer>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
}

impl Recorder {
    pub async fn new(
        current_player: String,
        app_handle: tauri::AppHandle,
        recording_consumer: Arc<RecordingConsumer>,
    ) -> Result<Self, anyhow::Error> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let ts = Timestamp::from_unix(NoContext, duration.as_secs(), duration.subsec_nanos());

        let id = Uuid::new_v7(ts);
        let session_id = format!("{}", id);

        let recording_path = app_handle
            .path()
            .app_local_data_dir()?
            .join("recordings")
            .join(&session_id);

        std::fs::create_dir_all(&recording_path)?;

        let manifest = SessionManifest {
            session_id: session_id.clone(),
            start_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64,
            end_timestamp: None,
            duration_ms: None,
            emitter_player: current_player.clone(),
            participants: Vec::new(),
            sample_rate: 48000,
            created_at: format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ),
        };

        Ok(Self {
            jobs: Vec::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
            session_id,
            manifest,
            recording_path,
            recording_consumer,
            app_handle,
        })
    }

    async fn start_recording_loop(&mut self) -> Result<AbortHandle, anyhow::Error> {
        const BATCH_SIZE: usize = 50;
        const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

        let recording_consumer = self.recording_consumer.clone();
        let shutdown = self.shutdown.clone();
        let recording_path = self.recording_path.clone();
        let mut manifest = self.manifest.clone();

        let handle = tokio::spawn(async move {
            let mut wal = match nano_wal::Wal::new(
                recording_path.join("wal").to_string_lossy().as_ref(),
                nano_wal::WalOptions::default(),
            ) {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to initialize WAL: {:?}", e);
                    return;
                }
            };

            let mut batch_buffer = Vec::new();
            let mut participants = HashSet::new();

            loop {
                tokio::select! {
                    recording_data = recording_consumer.recv_async() => {
                        match recording_data {
                            Ok(data) => {
                                if shutdown.load(Ordering::Relaxed) {
                                    break;
                                }

                                // Debug: Log received recording data
                                match &data {
                                    RecordingData::InputData { timestamp_ms, sample_rate, .. } => {
                                        info!("Received InputData: timestamp={}ms, sample_rate={}Hz", timestamp_ms, sample_rate);
                                    },
                                    RecordingData::OutputData { timestamp_ms, sample_rate, owner, .. } => {
                                        let unknown = String::from("unknown");
                                        let owner_name = owner.as_ref().map(|o| &o.name).unwrap_or(&unknown);
                                        info!("Received OutputData: timestamp={}ms, sample_rate={}Hz, owner={}", timestamp_ms, sample_rate, owner_name);
                                    },
                                }

                                // Track participants from output data
                                if let RecordingData::OutputData { owner, .. } = &data {
                                    if let Some(owner) = owner {
                                        participants.insert(owner.name.clone());
                                    }
                                }

                                // Determine type for batch processing
                                let data_type = match &data {
                                    RecordingData::InputData { .. } => "input",
                                    RecordingData::OutputData { .. } => "output",
                                };
                                batch_buffer.push((data_type, data));
                            }
                            Err(_) => continue,
                        }
                    }

                    _ = tokio::time::sleep(FLUSH_INTERVAL) => {
                        if !batch_buffer.is_empty() {
                            info!("💾 Flushing {} items to WAL (timeout)", batch_buffer.len());
                            if let Err(e) = Self::flush(&mut wal, &mut batch_buffer).await {
                                error!("Failed to flush batch during timeout: {:?}", e);
                                break;
                            }
                        }
                    }
                }

                if batch_buffer.len() >= BATCH_SIZE {
                    info!("💾 Flushing {} items to WAL (batch full)", batch_buffer.len());
                    if let Err(e) = Self::flush(&mut wal, &mut batch_buffer).await {
                        error!("Failed to flush batch when full: {:?}", e);
                        break;
                    }
                }

                if shutdown.load(Ordering::Relaxed) {
                    info!("Shutdown signal received, ending recording loop");
                    break;
                }
            }

            info!("Recording loop ending, performing final cleanup");

            if !batch_buffer.is_empty() {
                info!("💾 Final flush: {} items to WAL", batch_buffer.len());
                if let Err(e) = Self::flush(&mut wal, &mut batch_buffer).await {
                    error!("Failed final flush: {:?}", e);
                }
            }

            if let Err(e) = wal.sync() {
                error!("Failed final WAL sync: {:?}", e);
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            manifest.end_timestamp = Some(now);
            manifest.duration_ms = Some(now - manifest.start_timestamp);
            manifest.participants = participants.into_iter().collect();

            if let Ok(manifest_json) = serde_json::to_string_pretty(&manifest) {
                if let Err(e) = tokio::fs::write(
                    recording_path.join("session.json"),
                    manifest_json
                ).await {
                    error!("Failed to write session manifest: {:?}", e);
                } else {
                    info!("Session manifest written for recording {}", manifest.session_id);
                }
            }

            info!("Recording session {} fully finalized", manifest.session_id);
        });

        Ok(handle.abort_handle())
    }

    /// Flush WAL to disk using JSON serialization
    async fn flush(
        wal: &mut nano_wal::Wal,
        batch_buffer: &mut Vec<(&str, RecordingData)>,
    ) -> Result<(), anyhow::Error> {
        for (key, data) in batch_buffer.drain(..) {
            let serialized_data = serde_json::to_vec(&data)?;
            wal.append_entry(key, None, serialized_data.into(), false)?;
        }
        wal.sync()?;
        Ok(())
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl common::traits::StreamTrait for Recorder {
    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        self.shutdown.store(false, Ordering::Relaxed);

        let handle = self.start_recording_loop().await?;
        self.jobs.push(handle);

        info!("Recording session {} started", self.session_id);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        self.shutdown.store(true, Ordering::Relaxed);

        // Give recording loop 500ms to finish gracefully
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Then abort any remaining jobs
        for job in &self.jobs {
            job.abort();
        }

        info!("Recording session {} stopped", self.session_id);
        self.jobs.clear();
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.jobs.is_empty()
    }
}