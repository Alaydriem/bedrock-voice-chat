use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use anyhow::anyhow;

pub(crate) async fn stream_output(device: &cpal::Device) -> Result<(), anyhow::Error> {
    let config = device.default_output_config().unwrap();
    
    println!("{}", device.name().unwrap());
    match device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // react to stream events and read or write stream data here.
            },
            move |err| {
                // react to errors here.
            },
            None // None=blocking, Some(Duration)=timeout
        ) {
            Ok(stream) => { stream.play().unwrap(); loop {
                // If the server indicates we should mute, we should mute at the client level.
            } },
            Err(e) => Err(anyhow!("{}", e.to_string()))
        }
}

pub(crate) async fn stream_input(device: &cpal::Device) -> Result<(), anyhow::Error> {
    let config = device.default_input_config().unwrap();
    println!("{}", device.name().unwrap());
    match device.build_input_stream(
            &config.into(),
            move |data: & [f32], _: &cpal::InputCallbackInfo| {
                // react to stream events and read or write stream data here.
            },
            move |err| {
                // react to errors here.
            },
            None // None=blocking, Some(Duration)=timeout
        ) {
            Ok(stream) => { loop {} },
            Err(e) => Err(anyhow!("{}", e.to_string()))
        }
}

pub(crate) async fn get_devices(host: &cpal::platform::Host) -> Result<(Option<cpal::Device>, Option<cpal::Device>), anyhow::Error> {
    Ok((host.default_input_device(), host.default_output_device()))
}