use std::collections::hash_map::DefaultHasher;
use std::io::prelude::*;
use std::io::Read;
use std::mem::MaybeUninit;
use std::net::TcpStream;

use anyhow::anyhow;
use async_once_cell::OnceCell;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use cpal::StreamConfig;
use ringbuf::SharedRb;
use serde::{Deserialize, Serialize};
use std::hash::Hasher;

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioFrame {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
}

pub(crate) async fn stream_output(device: &cpal::Device) -> Result<(), anyhow::Error> {
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Default,
    };

    println!("{}", device.name().unwrap());
    let nsstream = std::net::TcpListener::bind("0.0.0.0:8444").unwrap();
    let c = config;
    let latency_frames = (50.0 / 1_000.0) * c.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * c.channels as usize;
    let ring = ringbuf::HeapRb::<f32>::new(latency_samples * 4);
    let (mut producer, mut consumer) = ring.split();
    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    let stream = match device.build_output_stream(
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
        None, // None=blocking, Some(Duration)=timeout
    ) {
        Ok(stream) => stream,
        Err(e) => return Err(anyhow!("{}", e.to_string())),
    };

    stream.play().unwrap();
    let mut total_size = 0;
    let mut raw_size = 0;
    let mut decoder = opus::Decoder::new(48000, opus::Channels::Mono).unwrap();
    for s in nsstream.incoming() {
        match s {
            Ok(mut stream) => {
                let mut size_buffer = [0u8; 8];
                match stream.peek(&mut size_buffer) {
                    Ok(_) => {
                        let packet_len = usize::from_be_bytes([
                            size_buffer[0],
                            size_buffer[1],
                            size_buffer[2],
                            size_buffer[3],
                            size_buffer[4],
                            size_buffer[5],
                            size_buffer[6],
                            size_buffer[7],
                        ]);
                        // The buffer for ron is the packet length
                        let mut buffer = vec![0; packet_len + 8];
                        stream.read(&mut buffer).unwrap();

                        total_size += buffer.len();
                        let decompressed = zstd::decode_all(&buffer[8..buffer.len()]).unwrap();
                        raw_size += decompressed.len() + 8;
                        let data =
                            std::str::from_utf8(&decompressed[0..decompressed.len()]).unwrap();

                        match ron::from_str::<AudioFrame>(data) {
                            Ok(frame) => {
                                let mut out = vec![0.0; 3840];
                                let out_len =
                                    match decoder.decode_float(&frame.data, &mut out, false) {
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
        };
    }

    println!("Exiting listener");
    Ok(())
}

pub(crate) async fn stream_input(device: &cpal::Device) -> Result<(), anyhow::Error> {
    // Input should be a mono channel
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Default,
    };

    println!("{}", device.name().unwrap());

    let latency_frames = (50.0 / 1_000.0) * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;
    let ring = ringbuf::HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();

    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    let stream = match device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for &sample in data {
                producer.push(sample).unwrap();
            }
        },
        move |err| {},
        None, // None=blocking, Some(Duration)=timeout
    ) {
        Ok(stream) => stream,
        Err(e) => return Err(anyhow!("{}", e.to_string())),
    };

    stream.play().unwrap();

    let mut data_stream = Vec::<f32>::new();
    #[allow(irrefutable_let_patterns)]
    while let sample = consumer.pop() {
        if sample.is_none() {
            continue;
        }
        data_stream.push(sample.unwrap());

        if data_stream.len() == 3840 {
            let mut mono = vec![0.0; data_stream.len()];
            for i in (0..data_stream.len()).step_by(2) {
                let average = (data_stream[i] / 2.0 + data_stream[i + 1] / 2.0) * 1.0;
                mono[i] = average;
                mono[i + 1] = average;
            }

            let dsc = mono.clone();
            data_stream = Vec::<f32>::new();

            let mut encoder = opus::Encoder::new(
                config.sample_rate.0.into(),
                opus::Channels::Mono,
                opus::Application::Voip,
            )
            .unwrap();

            let s = match encoder.encode_vec_float(&dsc, dsc.len() * 2) {
                Ok(s) => s,
                Err(e) => Vec::<u8>::with_capacity(0),
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

pub(crate) async fn get_devices(
    host: &cpal::platform::Host,
) -> Result<(Option<cpal::Device>, Option<cpal::Device>), anyhow::Error> {
    Ok((host.default_input_device(), host.default_output_device()))
}
