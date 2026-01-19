use serde::{Deserialize, Serialize};

fn default_broadcast_range() -> f32 {
    32.0
}

fn default_crouch_multiplier() -> f32 {
    1.0
}

fn default_whisper_multiplier() -> f32 {
    0.5
}

fn default_datagram_send_capacity() -> usize {
    1024
}

fn default_datagram_recv_capacity() -> usize {
    1024
}

/// Voice specific settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigVoice {
    #[serde(default = "default_broadcast_range")]
    pub broadcast_range: f32,
    #[serde(default = "default_crouch_multiplier")]
    pub crouch_distance_multiplier: f32,
    #[serde(default = "default_whisper_multiplier")]
    pub whisper_distance_multiplier: f32,
    /// Maximum number of outbound datagrams buffered per connection before backpressure / drops
    #[serde(default = "default_datagram_send_capacity")]
    pub datagram_send_capacity: usize,
    /// Maximum number of inbound datagrams buffered per connection
    #[serde(default = "default_datagram_recv_capacity")]
    pub datagram_recv_capacity: usize,
}

impl Default for ApplicationConfigVoice {
    fn default() -> Self {
        Self {
            broadcast_range: default_broadcast_range(),
            crouch_distance_multiplier: default_crouch_multiplier(),
            whisper_distance_multiplier: default_whisper_multiplier(),
            datagram_send_capacity: default_datagram_send_capacity(),
            datagram_recv_capacity: default_datagram_recv_capacity(),
        }
    }
}
