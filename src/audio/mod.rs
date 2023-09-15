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
    let config: cpal::StreamConfig = StreamConfig { channels: 2, sample_rate: SampleRate(48000), buffer_size: cpal::BufferSize::Fixed(960) };
    
    println!("{}", device.name().unwrap());
    let nsstream = std::net::TcpListener::bind("0.0.0.0:8444").unwrap();
    let c = config;
    let latency_frames = (10.0 / 1_000.0) * c.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * c.channels as usize;
    println!("{}", latency_samples);
    let ring = ringbuf::HeapRb::<f32>::new(1920);
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
                        Some(s) => { s },
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
        ) {
            Ok(stream) => stream,
            Err(e) => return Err(anyhow!("{}", e.to_string()))
        };
    
    stream.play().unwrap();
    for s in nsstream.incoming() {
        let mut rx_bytes = [0u8; 3840];
        let mut ss = s.unwrap();
        ss.read(&mut rx_bytes).unwrap();
        let bytes = convert_u8_to_f32(&rx_bytes);
        //println!("Recv: {:?}", bytes);

        // todo!() downsample to mono
        for sample in bytes {
                producer.push(sample.to_owned()).unwrap_or({});
        }
    }

    println!("Exiting listener");
    Ok(())
}

pub(crate) async fn stream_input(device: &cpal::Device) -> Result<(), anyhow::Error> {
    // Input should be a mono channel
    let config: cpal::StreamConfig = StreamConfig { channels: 2, sample_rate: SampleRate(48000), buffer_size: cpal::BufferSize::Fixed(960) };

    println!("{}", device.name().unwrap());

    let stream = match device.build_input_stream(
        &config,
        move |data: & [f32], _: &cpal::InputCallbackInfo| {
            let mut nsstream = TcpStream::connect("127.0.0.1:8444").unwrap();
            // We need a low & highpass filter, and a noise gate before transmitting
            let d = convert_f32_to_u8(&data);
            nsstream.write(d).unwrap();
            nsstream.flush().unwrap();
        },
        move |err| {
            // react to errors here.
            println!("{}", err.to_string());
        },
        None // None=blocking, Some(Duration)=timeout
    ) {
        Ok(stream) => stream,
        Err(e) => return Err(anyhow!("{}", e.to_string()))
    };

    stream.play().unwrap();
    loop {}
    println!("exiting input");
    Ok(())
}

pub(crate) async fn get_devices(host: &cpal::platform::Host) -> Result<(Option<cpal::Device>, Option<cpal::Device>), anyhow::Error> {
    Ok((host.default_input_device(), host.default_output_device()))
}

fn convert_f32_to_u8(input: &[f32]) -> &[u8] {
    // Assuming the input slice is of the same length
    let input_bytes = unsafe {
        std::slice::from_raw_parts(
            input.as_ptr() as *const u8,
            input.len() * std::mem::size_of::<f32>(),
        )
    };

    // Convert the bytes to &[u8]
    unsafe { std::slice::from_raw_parts(input_bytes.as_ptr(), input_bytes.len()) }
}

fn convert_u8_to_f32(input: &[u8]) -> &[f32] {
    // Assuming the input slice is of the same length
    let input_f32 = unsafe {
        std::slice::from_raw_parts(
            input.as_ptr() as *const f32,
            input.len() / std::mem::size_of::<f32>(),
        )
    };

    // Convert the bytes to &[f32]
    unsafe { std::slice::from_raw_parts(input_f32.as_ptr(), input_f32.len()) }
}