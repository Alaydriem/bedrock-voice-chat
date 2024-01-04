use std::io::prelude::*;
use std::io::Read;
use std::net::TcpStream;

use anyhow::anyhow;
use cpal::traits::{ DeviceTrait, HostTrait, StreamTrait };
use cpal::SampleRate;
use cpal::StreamConfig;
use serde::{ Deserialize, Serialize };
use audio_gate::NoiseGate;

const BUFFER_SIZE: u32 = 960;
#[derive(Debug, Deserialize, Serialize)]
pub struct AudioFrame {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
}
use common::ncryptflib as ncryptf;

#[tokio::main]
async fn main() {
    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();
    let output_device = host.default_output_device().unwrap();

    let kp = ncryptf::Keypair::new();
    let sk = ncryptf::Signature::new();
    let kp2 = kp.clone();
    let sk2 = sk.clone();
    let mut tasks = Vec::new();
    tasks.push(
        tokio::spawn(async move {
            _ = stream_input(&input_device, kp.clone(), sk.clone()).await;
        })
    );

    tasks.push(
        tokio::spawn(async move {
            _ = stream_output(&output_device, kp2.clone(), sk2.clone()).await;
        })
    );

    for task in tasks {
        _ = task.await;
    }
}

pub(crate) async fn stream_output(
    device: &cpal::Device,
    kp: common::ncryptflib::Keypair,
    sk: common::ncryptflib::Keypair
) -> Result<(), anyhow::Error> {
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
    };

    let mut decoder = opus::Decoder
        ::new(config.clone().sample_rate.0.into(), opus::Channels::Mono)
        .unwrap();

    let nsstream = std::net::TcpListener::bind("0.0.0.0:8444").unwrap();
    let c = config;
    let latency_frames = (250.0 / 1_000.0) * (c.sample_rate.0 as f32);
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

    for s in nsstream.incoming() {
        match s {
            Ok(mut stream) => {
                let mut size_buffer = [0u8; 8];
                match stream.peek(&mut size_buffer) {
                    Ok(_) => {
                        let now = std::time::Instant::now();
                        let packet_len = usize::from_be_bytes(
                            size_buffer[0..8].try_into().unwrap()
                        );
                        // The buffer for ron is the packet length
                        let mut buffer = vec![0; packet_len + 8];
                        stream.read(&mut buffer).unwrap();

                        let decompressed = zstd::decode_all(&buffer[8..buffer.len()]).unwrap();

                        let response = ncryptf::Response::from(kp.get_secret_key()).unwrap();
                        let ott = response
                            .decrypt(decompressed, Some(kp.get_public_key()), None)
                            .unwrap();
                        match ron::from_str::<AudioFrame>(ott.as_str()) {
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

                                println!(
                                    "Per Packet Decryption Time: {}",
                                    now.elapsed().as_micros()
                                );
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

pub(crate) async fn stream_input(
    device: &cpal::Device,
    kp: common::ncryptflib::Keypair,
    sk: common::ncryptflib::Keypair
) -> Result<(), anyhow::Error> {
    // Input should be a mono channel
    let config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(48000),
        buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
    };

    let mut encoder = opus::Encoder
        ::new(config.sample_rate.0.into(), opus::Channels::Mono, opus::Application::LowDelay)
        .unwrap();

    // Noise Gate Thresholds
    let mut gate = NoiseGate::new(-36.0, -54.0, 48000.0, 2, 150.0, 25.0, 150.0);

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
    #[allow(irrefutable_let_patterns)]
    while let sample = consumer.pop() {
        if sample.is_none() {
            continue;
        }
        data_stream.push(sample.unwrap());

        if data_stream.len() == (BUFFER_SIZE as usize) {
            let now = std::time::Instant::now();
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

            let mut req = ncryptf::Request::from(kp.get_secret_key(), sk.get_secret_key()).unwrap();
            let out = req.encrypt(raw.clone(), kp.get_public_key()).unwrap();

            let compressed = zstd::encode_all(out.as_slice(), 3).unwrap();

            println!("Per Packet Encryption time: {}", now.elapsed().as_micros());
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
