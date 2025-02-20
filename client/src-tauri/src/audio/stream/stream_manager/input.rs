use anyhow::anyhow;
use audio_gate::NoiseGate;
use common::structs::{
    audio::{AudioDevice, BUFFER_SIZE},
    packet::{AudioFramePacket, QuicNetworkPacket, QuicNetworkPacketData},
};
use log::{error, info, warn};
use opus::Bitrate;
use rodio::cpal::traits::StreamTrait;
use rodio::DeviceTrait;
use std::sync::{
    atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, Sender}, Arc
};
use tokio::task::{AbortHandle, JoinHandle};
use crate::{audio::stream::stream_manager::AudioFrameData, NetworkPacket};

use super::AudioFrame;

pub(crate) struct InputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Sender<NetworkPacket>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    metadata: Arc<moka::sync::Cache<String, String>>
}

impl super::StreamTrait for InputStream {
    fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        self.metadata.insert(key, value);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(true, Ordering::Relaxed);

        // Then hard terminate them
        for job in &self.jobs {
            job.abort();
        }

        self.jobs = vec![];
        Ok(())
    }

    fn is_stopped(&mut self) -> bool {
        self.jobs.len() == 0
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        let mut jobs = vec![];

        let (producer, consumer) = mpsc::channel();

        // Start the audio input listener thread
        match self.listener(producer, self.shutdown.clone()) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input listener encountered an error: {:?}", e);
                return Err(e);
            }
        };

        // Send the PCM data to the network sender
        match self.sender(consumer, self.shutdown.clone()) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input sender encountered an error: {:?}", e);
                return Err(e);
            }
        };

        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();
        Ok(())
    }
}

impl InputStream {
    pub fn new(device: Option<AudioDevice>, bus: Arc<flume::Sender<NetworkPacket>>) -> Self {
        Self {
            device,
            bus,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::sync::Cache::builder().build())
        }
    }

    // Produces raw PCM data and sends it to the network consumer
    fn listener(
        &mut self,
        producer: Sender<AudioFrame>,
        shutdown: Arc<AtomicBool>
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let cpal_device: Option<rodio::cpal::Device> = device.clone().into();

                    let handle = match cpal_device {
                        Some(cpal_device) => tokio::spawn(async move {
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
                                    _ = stream.play().unwrap();
                                    
                                    loop {
                                        if shutdown.load(Ordering::Relaxed) {
                                            stream.pause().unwrap();
                                            break;
                                        }
                                    }

                                    warn!("{} {} ended.", device.io.to_string(), device.display_name);
                                    drop(stream);
                                    return;
                                },
                                Err(e) => {
                                    error!("{:?}", e);
                                }
                            };
                        }),
                        None => {
                            error!("CPAL output device is not defined. This shouldn't happen! Restart BVC? {:?}", device.clone());
                            return Err(anyhow::anyhow!("Couldn't retrieve native cpal device for {} {}.", device.io.to_string(), device.display_name))
                        }
                    };

                    return Ok(handle);
                }
                Err(e) => return Err(e),
            },
            None => {
                return Err(anyhow!(
                    "InputStream is not initialized with a device! Unable to start stream"
                ))
            }
        };
    }

    fn sender(
        &mut self,
        consumer: Receiver<AudioFrame>,
        shutdown: Arc<AtomicBool>
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let device_config = rodio::cpal::StreamConfig {
                        channels: config.channels(),
                        sample_rate: config.sample_rate(),
                        buffer_size: cpal::BufferSize::Fixed(common::structs::audio::BUFFER_SIZE),
                    };

                    let mut data_stream = Vec::<f32>::new();

                    // Create the opus encoder
                    let mut encoder = match opus::Encoder::new(
                        device_config.sample_rate.0.into(),
                        opus::Channels::Mono,
                        opus::Application::Voip,
                    ) {
                        Ok(mut encoder) => {
                            _ = encoder.set_bitrate(Bitrate::Bits(64_000));
                            encoder
                        }
                        Err(e) => {
                            error!("Could not create opus encoder: {}", e.to_string());
                            return Err(anyhow!("{}", e.to_string()));
                        }
                    };

                    let bus = self.bus.clone();
                    let handle = tokio::spawn(async move {
                        #[allow(irrefutable_let_patterns)]
                        while let sample = consumer.recv() {
                            if shutdown.load(Ordering::Relaxed) {
                                break;
                            }
                            
                            match sample {
                                Ok(sample) => {
                                    let mut raw_sample = match sample.f32() {
                                        Some(sample) => sample.pcm,
                                        None => continue
                                    };

                                    data_stream.append(&mut raw_sample);
                                    if data_stream.len() >= (BUFFER_SIZE as usize) {
                                        let sample_to_process: Vec<f32> = data_stream
                                            .get(0..960)
                                            .unwrap()
                                            .to_vec();

                                        let mut remaining_data = data_stream
                                            .get(960..data_stream.len())
                                            .unwrap()
                                            .to_vec()
                                            .into_boxed_slice()
                                            .to_vec();
                                        data_stream = vec![0.0; 0];
                                        data_stream.append(&mut remaining_data);
                                        data_stream.shrink_to(data_stream.len());
                                        data_stream.truncate(data_stream.len());

                                        let s = match
                                            encoder.encode_vec_float(
                                                &sample_to_process,
                                                sample_to_process.len() * 4
                                            )
                                        {
                                            Ok(s) => s,
                                            Err(e) => {
                                                error!("{}", e.to_string());
                                                Vec::<u8>::with_capacity(0)
                                            }
                                        };

                                        // Opus frames with size of 3 or less mean that there either
                                        // was insufficient data to fill the buffer or an error
                                        // Buffer fill is intentional due to gates + other files
                                        // in the audio processing pipeline
                                        if s.len() <= 3 {
                                            continue;
                                        }

                                        let packet = NetworkPacket {
                                            data: QuicNetworkPacket {
                                                packet_type: common::structs::packet::PacketType::AudioFrame,
                                                owner: None, // This will be populated on the network side
                                                data: QuicNetworkPacketData::AudioFrame(AudioFramePacket {
                                                    length: s.len(),
                                                    data: s.clone(),
                                                    sample_rate: device_config.sample_rate.0,
                                                    coordinate: None,
                                                    dimension: None,
                                                    spatial: false // This will be mutated on the server
                                                })
                                            }
                                        };

                                        let tx = bus.clone();
                                        match tx.send(packet) {
                                            Ok(_) => {},
                                            Err(e) => {
                                                error!("Sending audio frame to Quic network thread failed: {:?}", e);
                                            }
                                        };
                                    }
                                },
                                Err(_e) => {
                                    // We're intentionaly supressing this error because the channel could not
                                    // yet be established -- since this is syncronous this'll throw constantly otherwise
                                }
                            }
                        }
                    });

                    return Ok(handle);
                }
                Err(e) => return Err(e),
            },
            None => {
                return Err(anyhow!(
                    "InputStream is not initialized with a device! Unable to start stream"
                ))
            }
        };
    }
}
