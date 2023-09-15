use std::io::prelude::*;
use std::io::Read;
use std::net::TcpStream;

use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use cpal::StreamConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioFrame {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<f32>,
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
    let latency_frames = (10.0 / 1_000.0) * c.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * c.channels as usize;
    let ring = ringbuf::HeapRb::<f32>::new(latency_samples * 2);
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
                        println!("{} {}", total_size, raw_size);
                        let data =
                            std::str::from_utf8(&decompressed[0..decompressed.len()]).unwrap();
                        match ron::from_str::<AudioFrame>(data) {
                            Ok(frame) => {
                                for sample in frame.data {
                                    producer.push(sample.to_owned()).unwrap_or({});
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

    let stream = match device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut nsstream = TcpStream::connect("127.0.0.1:8444").unwrap();
            // We need a low & highpass filter, and a noise gate before transmitting
            let af = AudioFrame {
                length: data.len(),
                data: data.to_owned(),
                sample_rate: config.sample_rate.0,
            };
            let raw = ron::to_string(&af).unwrap();
            let d = raw.as_bytes();

            let compressed = zstd::encode_all(&d[0..d.len()], 9).unwrap();
            let mut len = compressed.len().to_be_bytes().to_vec();
            len.extend_from_slice(&compressed);
            nsstream.write(&len).unwrap();
            nsstream.flush().unwrap();
        },
        move |err| {
            // react to errors here.
            println!("{}", err.to_string());
        },
        None, // None=blocking, Some(Duration)=timeout
    ) {
        Ok(stream) => stream,
        Err(e) => return Err(anyhow!("{}", e.to_string())),
    };

    stream.play().unwrap();
    loop {}
    println!("exiting input");
    Ok(())
}

pub(crate) async fn get_devices(
    host: &cpal::platform::Host,
) -> Result<(Option<cpal::Device>, Option<cpal::Device>), anyhow::Error> {
    Ok((host.default_input_device(), host.default_output_device()))
}
