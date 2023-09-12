use std::io::Read;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

use anyhow::anyhow;
use cpal::SampleRate;
use cpal::StreamConfig;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SizedSample,
};
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
        let mut rx_bytes = [0u8; 4];
        let mut ss = s.unwrap();
        ss.read(&mut rx_bytes).unwrap();
        producer.push(f32::from_be_bytes(rx_bytes)).unwrap();
    }

    println!("Exiting listener");
    Ok(())
}

pub(crate) async fn stream_input(device: &cpal::Device) -> Result<(), anyhow::Error> {
    // Input should be a mono channel
    let config: cpal::StreamConfig = StreamConfig { channels: 2, sample_rate: SampleRate(48000), buffer_size: cpal::BufferSize::Default };

    println!("{}", device.name().unwrap());

    let stream = match device.build_input_stream(
        &config,
        move |data: & [f32], _: &cpal::InputCallbackInfo| {
            let mut nsstream = TcpStream::connect("127.0.0.1:8444").unwrap();
            for &sample in data {
                // Send the sample packets to the server
                match nsstream.write(&sample.to_be_bytes()) {
                    Ok(_) => {
                    },
                    Err(e) => {
                        // An existing connection was forcibly closed by the remote host. (os error 10054)
                       println!("{}", e.to_string());
                    }
                };
                nsstream.flush().unwrap();
            }
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