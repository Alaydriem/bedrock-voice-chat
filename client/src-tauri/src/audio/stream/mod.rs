mod activity_detector;
pub mod jitter_buffer;
mod stream_manager;

use crate::audio::types::{AudioDevice, AudioDeviceType};
use crate::audio::recording::RecordingManager;
use crate::NetworkPacket;
use anyhow::Error;
use common::structs::audio::StreamEvent;
use std::sync::Arc;
use tauri::async_runtime::Mutex as TauriMutex;

use super::AudioPacket;
use stream_manager::{StreamTrait, StreamTraitType};

pub(crate) use activity_detector::ActivityUpdate;

pub(crate) struct AudioStreamManager {
    producer: Arc<flume::Sender<NetworkPacket>>,
    consumer: Arc<flume::Receiver<AudioPacket>>,
    input: StreamTraitType,
    output: StreamTraitType,
    app_handle: tauri::AppHandle,
    recording_manager: Option<Arc<TauriMutex<RecordingManager>>>,
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

        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                None,
                producer.clone(),
                Arc::new(moka::future::Cache::builder().build()),
                app_handle.clone(),
                None, // Producer will be set when initialized
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                None,
                consumer.clone(),
                Arc::new(moka::future::Cache::builder().build()),
                app_handle.clone(),
                None, // Producer will be set when initialized
            )),
            app_handle: app_handle.clone(),
            recording_manager,
        }
    }

    /// Initializes a given input or output stream with a specific device, then starts it
    pub async fn init(&mut self, device: AudioDevice) {
        // Stop the current stream if we're re-initializing a new one so we don't
        // have dangling thread pointers
        _ = self.stop(device.clone().io);

        // Get recording producer from manager if available
        let recording_producer = if let Some(ref rm) = self.recording_manager {
            let manager = rm.lock().await;
            Some(manager.get_producer())
        } else {
            None
        };

        match device.io {
            AudioDeviceType::InputDevice => {
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    Some(device),
                    self.producer.clone(),
                    self.input.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer,
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    Some(device),
                    self.consumer.clone(),
                    self.output.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer,
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

        // Get recording producer from manager if available
        let recording_producer = if let Some(ref rm) = self.recording_manager {
            let manager = rm.lock().await;
            Some(manager.get_producer())
        } else {
            None
        };

        match device {
            AudioDeviceType::InputDevice => {
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    self.input.get_device(),
                    self.producer.clone(),
                    self.input.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer,
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    self.output.get_device(),
                    self.consumer.clone(),
                    self.output.get_metadata().clone(),
                    self.app_handle.clone(),
                    recording_producer,
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
}
