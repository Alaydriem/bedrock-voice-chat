use std::io::prelude::*;
use std::io::Read;
use std::net::TcpStream;

use anyhow::Context;
use anyhow::anyhow;
use cpal::BufferSize;
use cpal::Device;
use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use cpal::StreamConfig;
use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize)]
pub struct AudioFrame {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<f32>,
    pub channels: u16,
    pub frame_size: u32,
    pub player: String,
    pub group: String
}

pub(crate) async fn stream_output(device: &cpal::Device) -> Result<(), anyhow::Error> {
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Default,
    };

    println!("{}", device.name().unwrap());
    // Create a persistent TCP Stream
    let nsstream = std::net::TcpListener::bind("0.0.0.0:8444").unwrap();

    // Setup a local ring buffer to pipe audio into from the incoming stream
    let latency_frames = (10.0 / 1_000.0) * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;
    let ring = ringbuf::HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();
    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    let stream = match device.build_output_stream(
        &config,
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

                        let decompressed = zstd::decode_all(&buffer[8..buffer.len()]).unwrap();

                        let data =
                            std::str::from_utf8(&decompressed[..]).unwrap();
                        
                        match ron::from_str::<AudioFrame>(data) {
                            Ok(frame) => {
                                for sample in frame.data {
                                    match producer.push(sample.to_owned()) {
                                        Ok(_) => {},
                                        Err(_) => {}
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
    let mut config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Default,
    };
    
    // 960 =    frame_size * channels * sizeof(float)
    // 480 = frame_size * sizeof(float)
    // 120 = frame_size
    println!("{}", device.name().unwrap());

    let stream = match device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Drop frames less than 960
            // todo!() add a ring buffer so we don't lose packets
            let input_frame_size = data.len();
            if input_frame_size != 960 { return; }

            let frame_size = (input_frame_size / config.channels as usize / 4) as u32;

            // Convert the stream into a single mono channel, sent as stereo
            let mut mono = vec![0.0; input_frame_size];
            for i in (0..input_frame_size).step_by(2) {
                let average = (data[i] / 2.0  + data[i+1] / 2.0) * 24.0;
                mono[i] = average;
                mono[i+1] = average;
            }

            let af = AudioFrame {
                length: mono.len(),
                data: mono,
                sample_rate: config.sample_rate.0,
                channels: config.channels,
                frame_size,
                player: "Alaydriem".to_string(),
                group: "Default".to_string()
            };

            let raw = ron::to_string(&af).unwrap();
            let d = raw.as_bytes();

            let compressed = zstd::encode_all(&d[0..d.len()], 13).unwrap();
            let mut buffer = compressed.len().to_be_bytes().to_vec();
            buffer.extend_from_slice(&compressed);

            let mut nsstream = TcpStream::connect("127.0.0.1:8444").unwrap();
            nsstream.write(&buffer).unwrap();
            nsstream.flush().unwrap();
            drop(nsstream);
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