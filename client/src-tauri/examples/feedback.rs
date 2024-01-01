use std::io::prelude::*;
use std::io::Read;
use std::net::TcpStream;

use anyhow::anyhow;
use cpal::traits::{ DeviceTrait, HostTrait, StreamTrait };
use cpal::SampleRate;
use cpal::StreamConfig;
use serde::{ Deserialize, Serialize };

const BUFFER_SIZE: u32 = 960;
#[derive(Debug, Deserialize, Serialize)]
pub struct AudioFrame {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();
    let output_device = host.default_output_device().unwrap();

    let mut tasks = Vec::new();
    tasks.push(
        tokio::spawn(async move {
            _ = stream_input(&input_device).await;
        })
    );

    tasks.push(
        tokio::spawn(async move {
            _ = stream_output(&output_device).await;
        })
    );

    for task in tasks {
        _ = task.await;
    }
}

pub(crate) async fn stream_output(device: &cpal::Device) -> Result<(), anyhow::Error> {
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
    };

    let nsstream = std::net::TcpListener::bind("0.0.0.0:8444").unwrap();
    let c = config;
    let latency_frames = (50.0 / 1_000.0) * (c.sample_rate.0 as f32);
    let latency_samples = (latency_frames as usize) * (c.channels as usize);
    let ring = ringbuf::HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();
    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    println!("{}", device.name().unwrap());

    let stream = match
        device.build_output_stream(
            &c,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // react to stream events and read or write stream data here.
                for sample in data {
                    *sample = match consumer.pop() {
                        Some(s) => s,
                        None => {
                            //println!("nothing");
                            0.0
                        }
                    };
                }
            },
            move |err| {
                // react to errors here.
            },
            None // None=blocking, Some(Duration)=timeout
        )
    {
        Ok(stream) => stream,
        Err(e) => {
            println!("Failed to build stream {}", e.to_string());
            return Err(anyhow!("{}", e.to_string()));
        }
    };

    stream.play().unwrap();

    let mut decoder = opus::Decoder::new(48000, opus::Channels::Mono).unwrap();
    for s in nsstream.incoming() {
        match s {
            Ok(mut stream) => {
                let mut size_buffer = [0u8; 8];
                match stream.peek(&mut size_buffer) {
                    Ok(_) => {
                        let packet_len = usize::from_be_bytes(
                            size_buffer[0..8].try_into().unwrap()
                        );
                        // The buffer for ron is the packet length
                        let mut buffer = vec![0; packet_len + 8];
                        stream.read(&mut buffer).unwrap();

                        let decompressed = zstd::decode_all(&buffer[8..buffer.len()]).unwrap();
                        let data = std::str
                            ::from_utf8(&decompressed[0..decompressed.len()])
                            .unwrap();

                        match ron::from_str::<AudioFrame>(data) {
                            Ok(frame) => {
                                let mut out = vec![0.0; BUFFER_SIZE as usize];
                                let out_len = match
                                    decoder.decode_float(&frame.data, &mut out, false)
                                {
                                    Ok(s) => s,
                                    Err(e) => {
                                        println!("{}", e.to_string());
                                        0
                                    }
                                };

                                out.truncate(out_len);

                                if out.len() > 0 {
                                    for sample in &out {
                                        producer.push(sample.to_owned()).unwrap_or({});
                                    }
                                }
                            }
                            Err(_) => {
                                println!("Unable to decode frame");
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
    }

    println!("Exiting listener");
    Ok(())
}

pub(crate) async fn stream_input(device: &cpal::Device) -> Result<(), anyhow::Error> {
    // Input should be a mono channel
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
    };

    println!("{}", device.name().unwrap());

    let latency_frames = (250.0 / 1_000.0) * (config.sample_rate.0 as f32);
    let latency_samples = (latency_frames as usize) * (config.channels as usize);
    let ring = ringbuf::HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();

    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    // Noise Gate Thresholds
    let mut gate = NoiseGate::new(-36.0, -54.0, 48000.0, 150.0, 25.0, 150.0);

    let stream = match
        device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let gated_data = gate.process_frame(&data);
                for &sample in gated_data.as_slice() {
                    producer.push(sample).unwrap_or({});
                }
            },
            move |err| {},
            None // None=blocking, Some(Duration)=timeout
        )
    {
        Ok(stream) => stream,
        Err(e) => {
            return Err(anyhow!("{}", e.to_string()));
        }
    };

    stream.play().unwrap();

    let mut data_stream = Vec::<f32>::new();
    let mut encoder = opus::Encoder
        ::new(config.sample_rate.0.into(), opus::Channels::Mono, opus::Application::LowDelay)
        .unwrap();
    #[allow(irrefutable_let_patterns)]
    while let sample = consumer.pop() {
        if sample.is_none() {
            continue;
        }
        data_stream.push(sample.unwrap());

        if data_stream.len() == (BUFFER_SIZE as usize) {
            let dsc = data_stream.clone();
            data_stream = Vec::<f32>::new();

            let s = match encoder.encode_vec_float(&dsc, dsc.len() * 2) {
                Ok(s) => s,
                Err(e) => {
                    println!("{}", e.to_string());
                    Vec::<u8>::with_capacity(0)
                }
            };

            if s.len() == 0 {
                continue;
            }

            // We need a low & highpass filter, and a noise gate before transmitting
            let af = AudioFrame {
                length: s.len(),
                data: s.clone(),
                sample_rate: config.sample_rate.0,
            };

            let raw = ron::to_string(&af).unwrap();
            let d = raw.as_bytes();

            let compressed = zstd::encode_all(&d[0..d.len()], 9).unwrap();
            let mut len = compressed.len().to_be_bytes().to_vec();
            len.extend_from_slice(&compressed);

            let mut nsstream = TcpStream::connect("127.0.0.1:8444").unwrap();
            nsstream.write(&len).unwrap();
            nsstream.flush().unwrap();
        }
    }
    println!("exiting input");
    Ok(())
}

#[derive(Debug, Clone)]
pub struct NoiseGate {
    open_threshold: f32,
    close_treshold: f32,
    sample_rate: f32,
    release_rate: f32,
    attack_rate: f32,
    decay_rate: f32,
    hold_time: f32,
    channels: usize,

    is_open: bool,
    attenuation: f32,
    level: f32,
    held_time: f32,
}

impl NoiseGate {
    pub fn new(
        open_threshold: f32,
        close_treshold: f32,
        sample_rate: f32,
        release_rate: f32,
        attack_rate: f32,
        hold_time: f32
    ) -> Self {
        let threshold_diff = open_threshold - close_treshold;
        let min_decay_period = (1.0 / 75.0) * sample_rate;

        Self {
            open_threshold: match open_threshold.is_finite() {
                true => (10_f32).powf(open_threshold / 20.0),
                false => 0.0,
            },
            close_treshold: match close_treshold.is_finite() {
                true => (10_f32).powf(close_treshold / 20.0),
                false => 0.0,
            },
            sample_rate: 1.0 / sample_rate,
            channels: 2,
            release_rate: 1.0 / (release_rate * 0.001 * sample_rate),
            attack_rate: 1.0 / (attack_rate * 0.001 * sample_rate),
            decay_rate: threshold_diff / min_decay_period,
            hold_time: hold_time * 0.001,
            is_open: false,
            attenuation: 0.0,
            level: 0.0,
            held_time: 0.0,
        }
    }

    pub fn process_frame(&mut self, frame: &[f32]) -> Vec<f32> {
        //return frame.into();
        let mut channel_frames = Vec::<Vec<f32>>::new();
        for _ in 0..self.channels {
            channel_frames.push(Vec::<f32>::with_capacity(frame.len() / self.channels));
        }

        for c in 0..self.channels {
            for (_, u) in frame.iter().enumerate().skip(c).step_by(self.channels) {
                channel_frames[c].push(*u);
            }
        }

        let mut resample = Vec::<f32>::with_capacity(frame.len());

        for i in 0..channel_frames[0].len() {
            let mut current_level = f32::abs(channel_frames[0][i]);

            for j in 0..self.channels {
                current_level = f32::max(current_level, channel_frames[j][i]);
            }

            if current_level > self.open_threshold && !self.is_open {
                self.is_open = true;
            }

            if self.level < self.close_treshold && self.is_open {
                self.held_time = 0.0;
                self.is_open = false;
            }

            self.level = f32::max(self.level, current_level) - self.decay_rate;

            if self.is_open {
                self.attenuation = f32::min(1.0, self.attenuation + self.attack_rate);
            } else {
                self.held_time += self.sample_rate;
                if self.held_time > self.hold_time {
                    self.attenuation = f32::max(0.0, self.attenuation - self.release_rate);
                }
            }

            for c in 0..self.channels {
                channel_frames[c][i] *= self.attenuation;
            }
        }

        // We need to flatten this back down to a single vec
        // For each channel
        // Grab the next element and push it to resample
        for i in 0..channel_frames[0].len() {
            for c in 0..self.channels {
                resample.push(channel_frames[c][i]);
            }
        }

        return resample.into();
    }
}
