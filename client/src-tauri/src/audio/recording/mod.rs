mod manager;
pub mod renderer;

use common::structs::recording::{RecordingPlayerData, SessionManifest, RecordingHeader, InputRecordingHeader, OutputRecordingHeader};

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

pub type RecordingProducer = flume::Sender<RawRecordingData>;
pub type RecordingConsumer = flume::Receiver<RawRecordingData>;


#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RawRecordingData {
    InputData {
        absolute_timestamp_ms: Option<u64>,
        opus_data: Vec<u8>,
        sample_rate: u32,
        channels: u16,
        emitter: RecordingPlayerData,
    },
    OutputData {
        absolute_timestamp_ms: Option<u64>,
        opus_data: Vec<u8>,
        sample_rate: u32,
        channels: u16,
        emitter: RecordingPlayerData,
        listener: RecordingPlayerData,
        is_spatial: bool,
    },
}

/// Core recorder that handles WAL storage and session management
pub struct Recorder {
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    session_id: String,
    manifest: SessionManifest,
    recording_path: PathBuf,
    recording_consumer: Arc<RecordingConsumer>,
    session_start_timestamp: u64,
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

        let start_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?;

        let manifest = SessionManifest {
            session_id: session_id.clone(),
            start_timestamp: start_timestamp.as_millis() as u64,
            end_timestamp: None,
            duration_ms: None,
            emitter_player: current_player.clone(),
            participants: Vec::new(),
            created_at: format!(
                "{}",
                start_timestamp
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
            session_start_timestamp: start_timestamp.as_millis() as u64,
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
        let session_start_timestamp = self.session_start_timestamp;

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

            // Write initial manifest
            if let Err(e) = Self::write_manifest(&recording_path, &manifest).await {
                error!("Failed to write initial manifest: {:?}", e);
            }

            let mut batch_buffer: Vec<(String, RawRecordingData)> = Vec::new();
            let mut participants = HashSet::new();
            let mut manifest_dirty = false;

            loop {
                tokio::select! {
                    raw_recording_data = recording_consumer.recv_async() => {
                        match raw_recording_data {
                            Ok(mut raw_data) => {
                                if shutdown.load(Ordering::Relaxed) {
                                    // Flush the batch buffer of anything we have
                                    let _ = Self::flush(&mut wal, &mut batch_buffer).await;
                                    break;
                                }

                                // Convert absolute timestamp to relative for WAL storage
                                // First packet becomes timestamp 0, all others relative to that
                                match &mut raw_data {
                                    RawRecordingData::InputData { absolute_timestamp_ms, .. } => {
                                        if let Some(abs_ts) = absolute_timestamp_ms {
                                            *absolute_timestamp_ms = Some(abs_ts.saturating_sub(session_start_timestamp));
                                        }
                                    },
                                    RawRecordingData::OutputData { absolute_timestamp_ms, emitter, .. } => {
                                        if let Some(abs_ts) = absolute_timestamp_ms {
                                            *absolute_timestamp_ms = Some(abs_ts.saturating_sub(session_start_timestamp));
                                        }

                                        // Track participants and mark manifest as dirty if new participant
                                        if participants.insert(emitter.name.clone()) {
                                            manifest_dirty = true;
                                        }
                                    }
                                }

                                let player_key = match &raw_data {
                                    RawRecordingData::InputData { emitter, .. } => emitter.name.clone(),
                                    RawRecordingData::OutputData { emitter, .. } => emitter.name.clone(),
                                };

                                batch_buffer.push((player_key, raw_data));
                            }
                            Err(_) => continue,
                        }
                    }

                    _ = tokio::time::sleep(FLUSH_INTERVAL) => {
                        if !batch_buffer.is_empty() {
                            if let Err(e) = Self::flush(&mut wal, &mut batch_buffer).await {
                                error!("Failed to flush batch during timeout: {:?}", e);
                                break;
                            }
                        }

                        // Write manifest if participants changed
                        if manifest_dirty {
                            manifest.participants = participants.iter().cloned().collect();
                            if let Err(e) = Self::write_manifest(&recording_path, &manifest).await {
                                error!("Failed to update manifest: {:?}", e);
                            } else {
                                manifest_dirty = false;
                            }
                        }
                    }
                }

                if batch_buffer.len() >= BATCH_SIZE {
                    if let Err(e) = Self::flush(&mut wal, &mut batch_buffer).await {
                        error!("Failed to flush batch when full: {:?}", e);
                        break;
                    }
                }

                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
            }

            if !batch_buffer.is_empty() {
                if let Err(e) = Self::flush(&mut wal, &mut batch_buffer).await {
                    error!("Failed final flush: {:?}", e);
                }
            }

            if let Err(e) = wal.sync() {
                error!("Failed final WAL sync: {:?}", e);
            }

            // Update manifest with final timestamp and duration
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            manifest.end_timestamp = Some(now);
            manifest.duration_ms = Some(now - manifest.start_timestamp);
            manifest.participants = participants.into_iter().collect();

            // Write final manifest
            if let Err(e) = Self::write_manifest(&recording_path, &manifest).await {
                error!("Failed to write final manifest: {:?}", e);
            }

            info!("Recording session {} fully finalized", manifest.session_id);
        });

        Ok(handle.abort_handle())
    }

    /// Write session manifest to disk
    async fn write_manifest(
        recording_path: &PathBuf,
        manifest: &SessionManifest,
    ) -> Result<(), anyhow::Error> {
        let manifest_json = serde_json::to_string_pretty(manifest)?;
        tokio::fs::write(
            recording_path.join("session.json"),
            manifest_json
        ).await?;
        Ok(())
    }

    /// Flush WAL to disk
    async fn flush(
        wal: &mut nano_wal::Wal,
        batch_buffer: &mut Vec<(String, RawRecordingData)>,
    ) -> Result<(), anyhow::Error> {
        for (player_key, data) in batch_buffer.drain(..) {
            // Create concrete headers with metadata and timestamps
            let header = match &data {
                RawRecordingData::InputData { sample_rate, channels, absolute_timestamp_ms, emitter, .. } => {
                    RecordingHeader::Input(InputRecordingHeader {
                        sample_rate: *sample_rate,
                        channels: *channels,
                        relative_timestamp_ms: *absolute_timestamp_ms,
                        emitter_metadata: emitter.to_metadata(),
                    })
                },
                RawRecordingData::OutputData { sample_rate, channels, absolute_timestamp_ms, emitter, listener, is_spatial, .. } => {
                    RecordingHeader::Output(OutputRecordingHeader {
                        sample_rate: *sample_rate,
                        channels: *channels,
                        relative_timestamp_ms: absolute_timestamp_ms.unwrap_or(0),
                        emitter_metadata: emitter.to_metadata(),
                        listener_metadata: listener.to_metadata(),
                        is_spatial: *is_spatial,
                    })
                }
            };

            let header_bytes = postcard::to_allocvec(&header)?;

            // Content is just the Opus data
            let content = match data {
                RawRecordingData::InputData { opus_data, .. } => opus_data,
                RawRecordingData::OutputData { opus_data, .. } => opus_data,
            };

            wal.append_entry(&player_key, Some(header_bytes.into()), content.into(), false)?;
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