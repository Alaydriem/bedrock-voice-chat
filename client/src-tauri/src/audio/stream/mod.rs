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
}

impl AudioStreamManager {
    /// Creates a new audio stream manager
    /// This is responsible for interfacing with all child threads
    pub fn new(
        producer: Arc<flume::Sender<NetworkPacket>>,
        consumer: Arc<flume::Receiver<AudioPacket>>,
    ) -> Self {
        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                None,
                producer.clone()
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                None,
                consumer.clone(),
            )),
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
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    Some(device),
                    self.consumer.clone(),
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
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    self.output.get_device(),
                    self.consumer.clone(),
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

    pub fn metadata(&mut self, key: String, value: String, device: &AudioDeviceType) -> Result<(), Error> {
        match device {
            AudioDeviceType::InputDevice => self.input.metadata(key, value),
            AudioDeviceType::OutputDevice => self.output.metadata(key, value)
        }
    }
}
