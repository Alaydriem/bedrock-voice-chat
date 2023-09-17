use std::io::prelude::*;
use std::io::Read;
use std::net::TcpStream;

use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;
use cpal::SizedSample;
use cpal::StreamConfig;
use cpal::{FromSample, Sample};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioFrame {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
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
                            std::str::from_utf8(&decompressed[0..decompressed.len()]).unwrap();
                        
                        match ron::from_str::<AudioFrame>(data) {
                            Ok(frame) => {
                                let frame_size = frame.frame_size;
                                let channels = frame.channels as u32;

                                // With a frame size of 120, the output buffer should be 960 bytes, which is the same as our original input size
                                let mut output = vec![0.0; (frame_size * channels * 4) as usize];

                                // this does not produce clear audio
                                match opus::Decoder::new(frame.sample_rate, opus::Channels::Stereo) {
                                    Ok(mut decoder) => match decoder.decode_float(&frame.data, &mut output, false){
                                        Ok(result) => {
                                            output.truncate(result);
                                            for sample in output {
                                                producer.push(sample.to_owned()).unwrap_or({});
                                            }
                                        },
                                        Err(e) => { println!("{}", e.to_string()); }
                                    },
                                    Err(e) => { println!("{}", e.to_string()); }
                                };

                                
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
    
    // 480 per channel at 48kHz, 20ms samples
    let frame_size = 120;

    // 960 = frame_size * channels * sizeof(float)
    // 480 = frame_size * sizeof(float)
    // 120 = frame_size
    println!("{}", device.name().unwrap());

    let mut lengths = Vec::<usize>::new();
    let mut encoder = opus::Encoder::new(config.sample_rate.0, opus::Channels::Stereo, opus::Application::Audio).unwrap();
    let stream = match device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {

            // Drop audio packets less than 960. Ideally we should throw this into a ring buffer then capture it
            if data.len() != 960 { return; }

            match encoder.encode_vec_float(&data, 2048) {
                // This is an single opus packet that we're sending across the network
                Ok(result) => {
                    // We need a low & highpass filter, and a noise gate before transmitting
                    let af = AudioFrame {
                        length: result.len(),
                        data: result,
                        sample_rate: config.sample_rate.0,
                        channels: config.channels,
                        frame_size,
                        player: "Alaydriem".to_string(),
                        group: "Default".to_string()
                    };

                    let raw = ron::to_string(&af).unwrap();
                    let d = raw.as_bytes();

                    let compressed = zstd::encode_all(&d[0..d.len()], 9).unwrap();
                    let mut buffer = compressed.len().to_be_bytes().to_vec();
                    buffer.extend_from_slice(&compressed);

                    let mut nsstream = TcpStream::connect("127.0.0.1:8444").unwrap();
                    nsstream.write(&buffer).unwrap();
                    nsstream.flush().unwrap();
                    drop(nsstream);
                    encoder.reset_state().unwrap();
                },
                Err(e) => { println!("{}", e.to_string()); }
            };
            encoder.reset_state().unwrap();
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

