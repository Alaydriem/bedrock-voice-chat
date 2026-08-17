mod recording;

pub use recording::RecordingConfig;

use common::structs::SpatialAudioConfig;
use serde::{Deserialize, Serialize};

fn default_datagram_send_capacity() -> usize {
    1024
}

fn default_datagram_recv_capacity() -> usize {
    1024
}

// Sits on the flat part of the measured curve (10p/10s: 27.8% of one core unbatched,
// 15.7% at 5ms, 12.6% at 10ms) while staying far inside the client's 120ms jitter
// depth — nothing under a frame period (20ms) is audible. 0 disables batching.
fn default_send_batch_wait_micros() -> u64 {
    7_500
}

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct Voice {
    // Maximum number of outbound datagrams buffered per connection before backpressure / drops
    #[serde(default = "default_datagram_send_capacity")]
    pub datagram_send_capacity: usize,
    // Maximum number of inbound datagrams buffered per connection
    #[serde(default = "default_datagram_recv_capacity")]
    pub datagram_recv_capacity: usize,
    // How long the per-connection sender waits after the first queued datagram for
    // more to arrive before flushing, in microseconds. 0 flushes each datagram as it
    // arrives. Non-zero trades up to that much added latency for fewer, fuller UDP
    // packets; the client jitter buffer absorbs values far below its depth.
    #[serde(default = "default_send_batch_wait_micros")]
    pub send_batch_wait_micros: u64,
    #[serde(default)]
    pub spatial_audio: SpatialAudioConfig,
    #[serde(default)]
    pub recording: RecordingConfig,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            datagram_send_capacity: default_datagram_send_capacity(),
            datagram_recv_capacity: default_datagram_recv_capacity(),
            send_batch_wait_micros: default_send_batch_wait_micros(),
            spatial_audio: SpatialAudioConfig::default(),
            recording: RecordingConfig::default(),
        }
    }
}
