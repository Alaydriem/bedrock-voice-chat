use std::{os::windows::process, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex, mpsc }, time::Duration};
use anyhow::anyhow;
use audio_gate::NoiseGate;
use log::{error, info, warn};
use rodio::DeviceTrait;
use tokio::{task::JoinHandle, time};
use common::structs::audio::AudioDevice;
use rodio::cpal::traits::StreamTrait;

use crate::{audio::stream::stream_manager::AudioFrameData, NetworkPacket};

use super::IpcMessage;
use super::AudioFrame;

pub(crate) struct InputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Sender<NetworkPacket>>,
    pub rx: spmc::Receiver<IpcMessage>,
    pub tx: spmc::Sender<IpcMessage>,
    pub producer: mpsc::Sender<AudioFrame>,
    pub consumer: mpsc::Receiver<AudioFrame>,
    pub shutdown: Arc<Mutex<AtomicBool>>,
    jobs: Vec<JoinHandle<()>>
}

impl super::StreamTrait for InputStream {
    fn stop(&mut self) {
        let shutdown = self.shutdown.clone();
        let shutdown = shutdown.lock().unwrap();
        shutdown.store(true, Ordering::Relaxed);
        _ = self.tx.send(IpcMessage::Terminate);

        // Give the threads time to detect that they should gracefully shut down
        _ = time::sleep(Duration::from_secs(1));
        
        // Then hard terminate them
        for job in &self.jobs {
            job.abort();
        }

        self.jobs = vec![];
    }

    fn is_stopped(&mut self) -> bool {
        let shutdown = self.shutdown.clone();
        let mut shutdown = shutdown.lock().unwrap();
        return shutdown.get_mut().to_owned();
    }

    fn start(&mut self) -> Result<(), anyhow::Error> {
        // We need to fetch the player name from Stronghold // Store (???)
        // How do we pull this in? Shouldn't it be initialized?

        // Start the audio input listener thread
        match self.listener() {
            Ok(_) => {},
            Err(e) => return Err(e)
        };

        // Send the PCM data to the network sender
        match self.sender() {
            Ok(_) => {},
            Err(e) => return Err(e)
        };

        Ok(())
    }
}

impl InputStream {
    pub fn new(
        device: Option<AudioDevice>,
        bus: Arc<flume::Sender<NetworkPacket>>
    ) -> Self {
        let (tx, rx) = spmc::channel();
        let (producer, consumer) = mpsc::channel();
        Self {
            device,
            bus,
            rx,
            tx,
            producer,
            consumer,
            shutdown: Arc::new(std::sync::Mutex::new(AtomicBool::new(false))),
            jobs: vec![]
        }
    }

    // Produces raw PCM data and sends it to the network consumer
    fn listener(&mut self) -> Result<(), anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    // Pull audio out of the device then producer.send() it
                    let producer = self.producer.clone();
                    let cpal_device: Option<rodio::cpal::Device> = device.clone().into();
                    
                    let audio_input_rx = self.rx.clone();
                    match cpal_device {
                        Some(cpal_device) => self.jobs.push(tokio::spawn(async move {
                            let rx = audio_input_rx.clone();
                            let cpal_device = cpal_device.clone();
                            let device = device.clone();
                            let device_config = rodio::cpal::StreamConfig {
                                channels: config.channels(),
                                sample_rate: config.sample_rate(),
                                buffer_size: cpal::BufferSize::Fixed(common::structs::audio::BUFFER_SIZE)
                            };

                            // @todo: Move this out of this thread and let it be configurable
                            let mut gate = NoiseGate::new(
                                -36.0,
                                -56.0,
                                device_config.sample_rate.0 as f32,
                                device_config.channels.into(),
                                150.0,
                                5.0,
                                150.0
                            );

                            let error_fn = move | error | {
                                warn!("an error occured on the stream: {}", error);
                            };

                            let mut process_fn = move | data: &[f32] | {
                                // @todo: Can this filter chain be called globally?
                                // Gate
                                let pcm = gate.process_frame(&data);
                                // Apply additional filters
                                // Supression
                                // Makeup limit
                                // Compressor
                                // Limiter
                                let audio_frame_data = AudioFrameData { pcm };
                                _ = producer.send(AudioFrame::F32(audio_frame_data)).unwrap();
                            };

                            let stream = match config.sample_format() {
                                rodio::cpal::SampleFormat::F32 => cpal_device.build_input_stream(
                                    &device_config,
                                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                        process_fn(&data);
                                    },
                                    error_fn,
                                    None
                                ),
                                rodio::cpal::SampleFormat::I32 => cpal_device.build_input_stream(
                                    &device_config,
                                    move |data: &[i32], _: &cpal::InputCallbackInfo| {
                                        const SCALE: f32 = 2147483648.0; // 2^31 to normalize properly
                                        let f32_data: Vec<f32> = data.iter().map(|&sample| sample as f32 / SCALE).collect();
                                        process_fn(&f32_data);
                                    },
                                    error_fn,
                                    None
                                ),
                                rodio::cpal::SampleFormat::I16 => cpal_device.build_input_stream(
                                    &device_config,
                                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                        const SCALE: f32 = 32768.0; // 2^15 to normalize properly
                                        let f32_data: Vec<f32> = data.iter().map(|&sample| sample as f32 / SCALE).collect();
                                        process_fn(&f32_data);
                                    },
                                    error_fn,
                                    None
                                ),
                                _ => {
                                    error!("{} {} does not have a supported sample format for streaming.", device.io.to_string(), device.display_name);
                                    return;
                                }
                            };

                            match stream {
                                Ok(stream) => {
                                    stream.play().unwrap();
                                    loop {
                                        let message: IpcMessage = rx.recv().unwrap();
                                        if message.eq(&IpcMessage::Terminate) {
                                            info!("{} {} Received shutdown signal, stopping audio stream.", device.io.to_string(), device.display_name);
                                            stream.pause().unwrap();
                                            break;
                                        }
                                    }

                                    info!("{} {} ended.", device.io.to_string(), device.display_name);
                                    drop(stream);
                                    return;
                                },
                                Err(e) => {
                                    error!("{:?}", e);
                                }
                            };
                        })),
                        None => {
                            error!("Couldn't retrieve native cpal device for {} {}.", device.io.to_string(), device.display_name);
                        }
                    }
                    
                    // consumer.recv(), then bus.send_async()
                    let bus = self.bus.clone();
                    self.jobs.push(tokio::spawn(async move {

                    }));
                },
                Err(e) => return Err(e)
            },
            None => return Err(anyhow!("InputStream is not initialized with a device! Unable to start stream"))
        };

        Ok(())
    }

    // 
    fn sender(&mut self) -> Result<(), anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let audio_input_rx = self.rx.clone();
                    let device_config = rodio::cpal::StreamConfig {
                        channels: config.channels(),
                        sample_rate: config.sample_rate(),
                        buffer_size: cpal::BufferSize::Fixed(common::structs::audio::BUFFER_SIZE)
                    };
                    let consumer = self.consumer;

                    self.jobs.push(tokio::spawn(async move {
                        while let sample = consumer.recv() {

                        }
                    }));
                },
                Err(e) => return Err(e)
            },
            None => return Err(anyhow!("InputStream is not initialized with a device! Unable to start stream"))
        };

        Ok(())
    }
}