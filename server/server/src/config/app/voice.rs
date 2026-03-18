use common::structs::SpatialAudioConfig;
use serde::{Deserialize, Serialize};

fn default_datagram_send_capacity() -> usize {
    1024
}

fn default_datagram_recv_capacity() -> usize {
    1024
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voice {
    // Maximum number of outbound datagrams buffered per connection before backpressure / drops
    #[serde(default = "default_datagram_send_capacity")]
    pub datagram_send_capacity: usize,
    // Maximum number of inbound datagrams buffered per connection
    #[serde(default = "default_datagram_recv_capacity")]
    pub datagram_recv_capacity: usize,
    #[serde(default)]
    pub spatial_audio: SpatialAudioConfig,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            datagram_send_capacity: default_datagram_send_capacity(),
            datagram_recv_capacity: default_datagram_recv_capacity(),
            spatial_audio: SpatialAudioConfig::default(),
        }
    }
}
