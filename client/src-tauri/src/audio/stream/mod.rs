mod stream_manager;

use crate::NetworkPacket;
use anyhow::Error;
use common::structs::audio::{AudioDevice, AudioDeviceType};
use std::sync::Arc;

use super::AudioPacket;
use stream_manager::{StreamTrait, StreamTraitType};

pub(crate) struct AudioStreamManager {
    producer: Arc<flume::Sender<NetworkPacket>>,
    consumer: Arc<flume::Receiver<AudioPacket>>,
    input: StreamTraitType,
    output: StreamTraitType,
    app_handle: tauri::AppHandle,
}

impl AudioStreamManager {
    /// Creates a new audio stream manager
    /// This is responsible for interfacing with all child threads
    pub fn new(
        producer: Arc<flume::Sender<NetworkPacket>>,
        consumer: Arc<flume::Receiver<AudioPacket>>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                None,
                producer.clone(),
                Arc::new(moka::future::Cache::builder().build()),
                app_handle.clone()
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                None,
                consumer.clone(),
                Arc::new(moka::future::Cache::builder().build()),
                app_handle.clone()
            )),
            app_handle: app_handle.clone()
        }
    }

    /// Initializes a given input or output stream with a specific device, then starts it
    pub fn init(&mut self, device: AudioDevice) {
        // Stop the current stream if we're re-initializing a new one so we don't
        // have dangling thread pointers
        _ = self.stop(device.clone().io);
        
        match device.io {
            AudioDeviceType::InputDevice => {
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    Some(device),
                    self.producer.clone(),
                    self.input.get_metadata().clone(),
                    self.app_handle.clone(),
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    Some(device),
                    self.consumer.clone(),
                    self.output.get_metadata().clone(),
                    self.app_handle.clone()
                ));
            }
        }
    }

    /// Restarts the audio stream for a given device
    /// This will stop the stream, create a new StreamManager with the same underlying device
    /// Then start a new stream in its place
    pub async fn restart(&mut self, device: AudioDeviceType) -> Result<(), Error> {
        // Stop the audio strema
        _ = self.stop(device.clone());
        
        match device {
            AudioDeviceType::InputDevice => {
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    self.input.get_device(),
                    self.producer.clone(),
                    self.input.get_metadata().clone(),
                    self.app_handle.clone()
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    self.output.get_device(),
                    self.consumer.clone(),
                    self.output.get_metadata().clone(),
                    self.app_handle.clone()
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
                )))
            },
            AudioDeviceType::OutputDevice => match self.output.is_stopped() {
                true => self.output.start().await,
                false => Err(anyhow::anyhow!(format!(
                    "{} audio stream is already running!",
                    device.to_string()
                )))
            }
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

    pub async fn metadata(&mut self, key: String, value: String, device: &AudioDeviceType) -> Result<(), Error> {
        match device {
            AudioDeviceType::InputDevice => self.input.metadata(key, value).await,
            AudioDeviceType::OutputDevice => self.output.metadata(key, value).await
        }
    }

    pub async fn mute(&mut self, device: &AudioDeviceType) -> Result<(), Error> {
        match device {
            AudioDeviceType::InputDevice => self.input.mute(),
            AudioDeviceType::OutputDevice => self.output.mute()
        };

        Ok(())
    }

    pub async fn mute_status(&mut self, device: &AudioDeviceType) -> Result<bool, Error> {
        let status = match device {
            AudioDeviceType::InputDevice => self.input.mute_status(),
            AudioDeviceType::OutputDevice => self.output.mute_status()
        };

        Ok(status)
    }
}
