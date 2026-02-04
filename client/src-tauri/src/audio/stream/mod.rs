mod activity_detector;
pub mod jitter_buffer;
mod stream_manager;

use crate::audio::types::{AudioDevice, AudioDeviceType};
use crate::audio::recording::RecordingManager;
use crate::NetworkPacket;
use anyhow::Error;
use common::structs::audio::StreamEvent;
use log::warn;
use std::sync::Arc;
use tauri::async_runtime::Mutex as TauriMutex;
use tauri::Emitter;
use tokio::sync::mpsc;

use super::AudioPacket;
use stream_manager::{StreamTrait, StreamTraitType};

pub(crate) use activity_detector::ActivityUpdate;

/// Event sent when a stream encounters an error requiring recovery
#[derive(Debug, Clone)]
pub enum StreamRecoveryEvent {
    DeviceError { device_type: AudioDeviceType, error: String },
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
    /// Receiver for recovery events - consumed when monitor is spawned
    recovery_rx: Option<mpsc::UnboundedReceiver<StreamRecoveryEvent>>,
}

impl AudioStreamManager {
    /// Creates a new audio stream manager
    /// This is responsible for interfacing with all child threads
    pub fn new(
        producer: Arc<flume::Sender<NetworkPacket>>,
        consumer: Arc<flume::Receiver<AudioPacket>>,
        app_handle: tauri::AppHandle,
        recording_manager: Option<Arc<TauriMutex<RecordingManager>>>,
    ) -> Self {
        // Producer will be extracted when streams are initialized
        // to avoid blocking the setup function

        // Create recovery channel for error handling
        // The receiver is stored and the monitor task is spawned lazily
        // when init() is first called (from an async context)
        let (recovery_tx, recovery_rx) = mpsc::unbounded_channel::<StreamRecoveryEvent>();

        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                None,
                producer.clone(),
                Arc::new(moka::future::Cache::builder().build()),
                app_handle.clone(),
                None, // Producer will be set when initialized
                None, // Recording flag will be set when initialized
                recovery_tx.clone(),
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                None,
                consumer.clone(),
                Arc::new(moka::future::Cache::builder().build()),
                app_handle.clone(),
                None, // Producer will be set when initialized
                None, // Recording flag will be set when initialized
                recovery_tx.clone(),
            )),
            app_handle: app_handle.clone(),
            recording_manager,
            recovery_tx,
            recovery_rx: Some(recovery_rx),
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
                            warn!("Stream recovery triggered for {:?}: {}", device_type, error);
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

    /// Initializes a given input or output stream with a specific device, then starts it
    pub async fn init(&mut self, device: AudioDevice) {
        // Spawn recovery monitor on first init (now we're in async context)
        self.spawn_recovery_monitor();

        // Stop the current stream if we're re-initializing a new one so we don't
        // have dangling thread pointers
        _ = self.stop(device.clone().io);

        // Get recording producer and flag from manager if available
        let (recording_producer, recording_flag) = if let Some(ref rm) = self.recording_manager {
            let manager = rm.lock().await;
            (Some(manager.get_producer()), Some(manager.get_recording_flag()))
        } else {
            (None, None)
        };

        match device.io {
            AudioDeviceType::InputDevice => {
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    Some(device),
                    self.producer.clone(),
                    self.input.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer.clone(),
                    recording_flag.clone(),
                    self.recovery_tx.clone(),
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    Some(device),
                    self.consumer.clone(),
                    self.output.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer,
                    recording_flag,
                    self.recovery_tx.clone(),
                ));
            }
        }
    }

    /// Restarts the audio stream for a given device
    /// This will stop the stream, create a new StreamManager with the same underlying device
    /// Then start a new stream in its place
    #[allow(unused)]
    pub async fn restart(&mut self, device: AudioDeviceType) -> Result<(), Error> {
        // Stop the audio strema
        _ = self.stop(device.clone());

        // Get recording producer and flag from manager if available
        let (recording_producer, recording_flag) = if let Some(ref rm) = self.recording_manager {
            let manager = rm.lock().await;
            (Some(manager.get_producer()), Some(manager.get_recording_flag()))
        } else {
            (None, None)
        };

        match device {
            AudioDeviceType::InputDevice => {
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    self.input.get_device(),
                    self.producer.clone(),
                    self.input.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer.clone(),
                    recording_flag.clone(),
                    self.recovery_tx.clone(),
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    self.output.get_device(),
                    self.consumer.clone(),
                    self.output.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer,
                    recording_flag,
                    self.recovery_tx.clone(),
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
                    device.to_string()
                ))),
            },
            AudioDeviceType::OutputDevice => match self.output.is_stopped() {
                true => self.output.start().await,
                false => Err(anyhow::anyhow!(format!(
                    "{} audio stream is already running!",
                    device.to_string()
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
        match device {
            AudioDeviceType::InputDevice => self.input.metadata(key, value).await,
            AudioDeviceType::OutputDevice => self.output.metadata(key, value).await,
        }
    }

    pub async fn toggle(&mut self, device: &AudioDeviceType, event: StreamEvent) -> Result<(), Error> {
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

    /// Resets the audio stream manager by stopping all streams and recreating them
    /// This is used when a full reset is needed (e.g., after page refresh)
    pub async fn reset(&mut self) -> Result<(), Error> {
        // Stop both streams concurrently
        let (_, _) = tokio::join!(
            self.input.stop(),
            self.output.stop()
        );

        // Get recording producer and flag from manager if available
        let (recording_producer, recording_flag) = if let Some(ref rm) = self.recording_manager {
            let manager = rm.lock().await;
            (Some(manager.get_producer()), Some(manager.get_recording_flag()))
        } else {
            (None, None)
        };

        // Recreate input stream, preserving metadata
        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            None,
            self.producer.clone(),
            self.input.get_metadata().clone(),
            self.app_handle.clone(),
            recording_producer.clone(),
            recording_flag.clone(),
            self.recovery_tx.clone(),
        ));

        // Recreate output stream, preserving metadata
        self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
            None,
            self.consumer.clone(),
            self.output.get_metadata().clone(),
            self.app_handle.clone(),
            recording_producer,
            recording_flag,
            self.recovery_tx.clone(),
        ));

        Ok(())
    }

    /// Returns the list of currently tracked players from the output stream's presence cache
    pub fn get_current_players(&self) -> Vec<String> {
        match &self.output {
            StreamTraitType::Output(stream) => stream.get_current_players(),
            _ => vec![],
        }
    }
}
