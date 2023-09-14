use std::io::prelude::*;
use std::io::Read;
use std::net::TcpStream;

use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Sample;
use cpal::SampleRate;
use cpal::StreamConfig;

use opus;

#[derive(Copy, Clone)]
pub(crate) enum AudioSampleRates {
    Sr44100 = 1,
    Sr48000 = 2,
}

impl AudioSampleRates {
    /// Returns the frame size for stereo we expect for each sample rate
    pub fn get_frame_size(&self) -> u32 {
        match self {
            Self::Sr44100 => 880,
            Self::Sr48000 => 960,
        }
    }
}

impl From<u32> for AudioSampleRates {
    fn from(input: u32) -> Self {
        match input {
            44100 => AudioSampleRates::Sr44100,
            48000 => AudioSampleRates::Sr48000,
            _ => AudioSampleRates::Sr48000,
        }
    }
}

impl From<AudioSampleRates> for u32 {
    fn from(input: AudioSampleRates) -> Self {
        match input {
            AudioSampleRates::Sr44100 => 44100,
            AudioSampleRates::Sr48000 => 48000,
        }
    }
}

pub(crate) async fn stream_output(device: &cpal::Device) -> Result<(), anyhow::Error> {
    let config = device.default_output_config().unwrap();

    println!("{}", device.name().unwrap());
    let nsstream = std::net::TcpListener::bind("0.0.0.0:8444").unwrap();
    let c = config.config();
    let latency_frames = (300.0 / 1_000.0) * c.sample_rate.0 as f32;
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
            println!("{}", err.to_string());
        },
        None, // None=blocking, Some(Duration)=timeout
    ) {
        Ok(stream) => stream,
        Err(e) => return Err(anyhow!("{}", e.to_string())),
    };

    stream.play().unwrap();
    for stream in nsstream.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut size_buffer = [0u8; 2];
                match stream.peek(&mut size_buffer) {
                    Ok(_) => {
                        let compressed_opus_len = size_buffer[0] as usize;
                        let sample_length = AudioSampleRates::from(size_buffer[1] as u32);
                        let mut buffer = vec![0; compressed_opus_len];

                        // Fill the buffer with the stream
                        stream.read(&mut buffer).unwrap();

                        //dbg!("Frame Size: {}", sample_length.get_frame_size());
                        let mut output: Vec<f32> =
                            vec![0.0; sample_length.get_frame_size() as usize];
                        //dbg!("{}", output.len());
                        match opus::Decoder::new(sample_length.into(), opus::Channels::Stereo) {
                            Ok(mut decoder) => match decoder.decode_float(
                                &buffer[2..compressed_opus_len],
                                &mut output,
                                false,
                            ) {
                                Ok(size) => {
                                    println!("{:?} {:?}", size, output.len());
                                    // size is the number of samples per channel
                                    for sample in &output {
                                        match producer.push(sample.to_owned()) {
                                            Ok(_) => {}
                                            Err(_) => {}
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("{:?}", e)
                                }
                            },
                            Err(_) => {}
                        }
                    }
                    Err(_) => {}
                };
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
        buffer_size: cpal::BufferSize::Default,
    };

    println!("{}", device.name().unwrap());

    let stream = match device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // 48kHz => 480 samples per channel, 960 per frame; 1920 bytes for Mono, 3840 for stereo
            //println!("\nsent {:?}", &data.len());
            let sample_rate_e: AudioSampleRates = config.sample_rate.0.into();
            let mut nsstream = TcpStream::connect("127.0.0.1:8444").unwrap();
            // We need a low & highpass filter, and a noise gate before transmitting
            match opus::Encoder::new(
                sample_rate_e.into(),
                opus::Channels::Stereo,
                opus::Application::Audio,
            ) {
                Ok(mut encoder) => match encoder.encode_vec_float(&data[0..960], 960) {
                    Ok(mut result) => {
                        let size = result.len() + 2;
                        let mut ds: Vec<u8> = vec![size as u8];
                        ds.append(&mut vec![sample_rate_e as u8]);
                        ds.append(&mut result);

                        nsstream.write(ds.as_ref()).unwrap();
                        nsstream.flush().unwrap();
                    }
                    Err(e) => {
                        println!("unable to encode {:?}", e);
                    }
                },
                Err(e) => {
                    println!("sending error {}", e.to_string());
                }
            };
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
}

pub(crate) async fn get_devices(
    host: &cpal::platform::Host,
) -> Result<(Option<cpal::Device>, Option<cpal::Device>), anyhow::Error> {
    Ok((host.default_input_device(), host.default_output_device()))
}
