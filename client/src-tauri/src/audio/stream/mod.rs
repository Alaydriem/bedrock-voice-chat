mod stream_manager;

use anyhow::Error;
use common::structs::audio::{AudioDevice, AudioDeviceType};
use crate::NetworkPacket;
use std::sync::Arc;

use super::AudioPacket;
use stream_manager::{StreamTrait, StreamTraitType};

pub(crate) struct AudioStreamManager {
    producer: Arc<flume::Sender<NetworkPacket>>,
    consumer: Arc<flume::Receiver<AudioPacket>>,
    input: StreamTraitType,
    output: StreamTraitType
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
                consumer.clone()
            ))
        }
    }

    /// Initializes a given input or output stream with a specific device, then starts it
    pub fn init(&mut self, device: AudioDevice) {
        match device.io {
            AudioDeviceType::InputDevice => {
                self.input =  StreamTraitType::Input(stream_manager::InputStream::new(
                    Some(device),
                    self.producer.clone()
                ));
            }
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    Some(device),
                    self.consumer.clone()
                ));
            }
        }
    }

    /// Restarts the audio stream for a given device
    /// This will stop the stream, create a new StreamManager with the same underlying device
    /// Then start a new stream in its place
    pub async fn restart(&mut self, device: &AudioDeviceType) -> Result<(), Error> {
        // Stop the audio strema
        _ = self.stop(device);
        match device {
            AudioDeviceType::InputDevice => {
                self.input = StreamTraitType::Input(stream_manager::InputStream::new(
                    Some(self.input.get_device().unwrap()),
                    self.producer.clone()
                ));
            },
            AudioDeviceType::OutputDevice => {
                self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
                    Some(self.output.get_device().unwrap()),
                    self.consumer.clone()
                ));
            }
        };

        self.start(device).await
    }

    /// Starts the stream for a given audio device type
    pub async fn start(&mut self, device: &AudioDeviceType) -> Result<(), Error> {
        // Start the new device
        let _ = match device {
            AudioDeviceType::InputDevice => match self.input.is_stopped() {
                true => return Err(anyhow::anyhow!(format!("{} audio stream is already running!", device.to_string()))),
                false => self.input.start()
            },
            AudioDeviceType::OutputDevice => match self.output.is_stopped() {
                true => return Err(anyhow::anyhow!(format!("{} audio stream is already running!", device.to_string()))),
                false => self.output.start()
            }
        };

        Ok(())
    }

    /// Stops the audio stream for the given device
    /// This permanently shuts down all associated threads
    /// To restart the device, either call restart(), or re-initialize the device
    pub fn stop(&mut self, device: &AudioDeviceType) -> Result<(), Error> {
        match device {
            AudioDeviceType::InputDevice => match self.input.is_stopped() {
                true => return Err(anyhow::anyhow!(format!("{} audio stream is already stopped!", device.to_string()))),
                false => self.input.stop()
            },
            AudioDeviceType::OutputDevice => match self.output.is_stopped() {
                true => return Err(anyhow::anyhow!(format!("{} audio stream is already stopped!", device.to_string()))),
                false => self.output.stop()
            }
        };

        Ok(())
    }
}