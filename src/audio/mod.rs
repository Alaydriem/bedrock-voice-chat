use std::io::prelude::*;
use std::io::Read;
use std::net::TcpStream;

use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Sample;
use cpal::SampleRate;
use cpal::StreamConfig;

use opus;

#[derive(Debug, Copy, Clone)]
pub(crate) enum AudioSampleRates {
    Sr44100 = 1,
    Sr48000 = 2,
}

impl AudioSampleRates {
    pub fn new(i: u8) -> Self {
        match i {
            1 => Self::Sr44100,
            2 => Self::Sr48000,
            _ => Self::Sr48000,
        }
    }

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
    let config = device.default_input_config().unwrap();
    let sample_config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        // Use the device sample rate, but it should be 48kHz. Enforce?
        sample_rate: config.sample_rate(),
        // 48kHz => 480 samples per channel, 960 per frame; 1920 bytes for Mono, 3840 for stereo
        // 1920 bytes is per-channel - the data callback will be double (3840 at 48kHz)
        // We want a fixed size so we can pack it into an Opus frame evenly
        // Opus wants the frame length to be sizeof(float) * channel * frame_size
        // If our input is 3840, the opus:: crate wil set the frame size to 960 per channel, which'll give us a 10ms frame_size
        buffer_size: cpal::BufferSize::Fixed(1920),
    };

    println!("{}", device.name().unwrap());
    let nsstream = std::net::TcpListener::bind("0.0.0.0:8444").unwrap();
    let ring = ringbuf::HeapRb::<f32>::new(3840);
    let (mut producer, mut consumer) = ring.split();
    for _ in 0..3840 {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    let stream = match device.build_output_stream(
        &sample_config,
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
                let mut size_buffer = [0u8; 10];
                match stream.peek(&mut size_buffer) {
                    Ok(_) => {
                        let sb = [
                            size_buffer[0],
                            size_buffer[1],
                            size_buffer[2],
                            size_buffer[3],
                            size_buffer[4],
                            size_buffer[5],
                            size_buffer[6],
                            size_buffer[7],
                        ];
                        let compressed_opus_len = usize::from_be_bytes(sb);
                        let sample_length = AudioSampleRates::new(size_buffer[8]);
                        let mut buffer = vec![0; compressed_opus_len];

                        // Fill the buffer with the stream
                        stream.read(&mut buffer).unwrap();

                        let mut output: Vec<f32> =
                            vec![0.0; (sample_length.get_frame_size() as usize) * 4];

                        println!(
                            "rec {} => {} | {:?}",
                            compressed_opus_len,
                            output.len(),
                            sample_length
                        );

                        match opus::Decoder::new(sample_length.into(), opus::Channels::Stereo) {
                            Ok(mut decoder) => match decoder.decode_float(
                                &buffer[9..compressed_opus_len],
                                &mut output,
                                false,
                            ) {
                                Ok(size) => {
                                    //println!("{:?} {:?}", size, output.len());
                                    // size is the number of samples per channel
                                    for sample in output {
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
    let config = device.default_input_config().unwrap();
    let sample_config: cpal::StreamConfig = StreamConfig {
        channels: 2,
        // Use the device sample rate, but it should be 48kHz. Enforce?
        sample_rate: config.sample_rate(),
        // 48kHz => 480 samples per channel, 960 per frame; 1920 bytes for Mono, 3840 for stereo
        // 1920 bytes is per-channel - the data callback will be double (3840 at 48kHz)
        // We want a fixed size so we can pack it into an Opus frame evenly
        // Opus wants the frame length to be sizeof(float) * channel * frame_size
        // If our input is 3840, the opus:: crate wil set the frame size to 960 per channel, which'll give us a 10ms frame_size
        buffer_size: cpal::BufferSize::Fixed(1920),
    };

    println!("{}", device.name().unwrap());

    let stream = match device.build_input_stream(
        &sample_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            //println!("\nsent {:?}", &data.len());
            let sample_rate_e: AudioSampleRates = sample_config.sample_rate.0.into();
            let mut nsstream = TcpStream::connect("127.0.0.1:8444").unwrap();
            // We need a low & highpass filter, and a noise gate before transmitting
            match opus::Encoder::new(
                sample_rate_e.into(),
                opus::Channels::Stereo,
                opus::Application::Audio,
            ) {
                Ok(mut encoder) => match encoder.encode_vec_float(&data, data.len() * 2) {
                    Ok(mut result) => {
                        let size = result.len() + 9;
                        let sr = (sample_rate_e as u32) as u8;
                        let mut size_buffer = size.to_be_bytes().to_vec();
                        size_buffer.push(sr);
                        size_buffer.extend(result);

                        println!("Sent {} => {}", data.len(), size);
                        nsstream.write(size_buffer.as_ref()).unwrap();
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
