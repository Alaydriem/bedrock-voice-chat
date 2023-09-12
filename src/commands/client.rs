use clap::Parser;
use std::sync::Arc;
use super::Command;
use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait};
use crate::audio;
#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    // todo!() Conver this to be an Xbox Live authentication request
    /// Your Xbox Live Gamertag
    #[clap(long, required = true)]
    pub gamertag: String,

    /// The bedrock voice chat server (host:port) you want to connect to
    #[clap(long, required = true)]
    pub server: String
}

impl Config {
    pub async fn run<'a>(&'a self, _cfg: &Arc<Command>) {
        /*
        let (username, id) = match crate::auth::auth().await {
            Ok((u, i)) => (u, i),
            Err(e) => panic!("{}", e.to_string())
        };
        */

        let host: cpal::platform::Host;
        #[cfg(target_os = "windows")]
        {
            //host = cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialise ASIO host");
            host = cpal::host_from_id(cpal::HostId::Wasapi).expect("failed to initialise ASIO host");
        }

        // Default to the system input devices
        // todo!() Allow the user to select from either WASAPI or ASIO, or as command line arguments.
        let (input, output) = match audio::get_devices(&host).await {
            Ok((i, o)) => {
                if i.is_some() && o.is_some() {
                    (i.unwrap(), o.unwrap())
                } else {
                    panic!("Default i/o not working.")
                }
            },
            Err(e) => panic!("{}", e.to_string())
        };
        
        let mut tasks = Vec::new();

        // Create a task to get everyone on the server, the port they are streaming to, their position data, and any user settings
        // This should loop every 3-5 seconds or so
        // This tasks should also monitor all of the active streams, and if a new one is added that we don't know about, add a new task for it
        // And remove tasks that are no longer present, based upon the group.

        // All actors are put into a group, with the "default" being "everyone", meaning everyone can hear everyone.
        // If the group changes, all streams but the output streams of the specific users should be terminated so we aren't wasting bandwidth
        // We need some global client state to track all of this in memory.
        // And to figure out how to secure this. QUIC? https://github.com/aws/s2n-quic ?

        // Start recording on the input device and stream it to the server
        // We're going to always push this to the server
        let input_stream = tokio::spawn(async move {
            match audio::stream_input(&input).await {
                Ok(r) => println!("did something"),
                Err(e) => println!("input {}", e.to_string())
            }
        });
        tasks.push(input_stream);

        // Spawn an output stream with the device we have
        let output_stream = tokio::spawn(async move {
            match audio::stream_output(&output).await {
                Ok(r) => println!("did something"),
                Err(e) => println!("output {}", e.to_string())
            }
        });
        tasks.push(output_stream);

        for task in tasks {
            #[allow(unused_must_use)]
            {
                task.await;
            }
        }

        loop {};
    }
}
