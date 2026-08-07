use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::audio::NoiseGateStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct MicDiagnostics {
    pub device: Option<String>,
    pub sample_rate: Option<u32>,
    // Read from the flag the capture path itself consults, combined with whether any
    // captured frame carried signal during the interval. Sampled over an interval rather
    // than instantaneously: at a 20 ms frame cadence a single reading lands on a
    // near-random frame and flickers.
    pub noise_gate: NoiseGateStatus,
    pub muted: bool,
    // Frames arriving from the capture device, before the gate, the encoder or the network.
    // The one measurement that separates a microphone that stopped from a microphone whose
    // audio stopped being delivered, and the one number a stopped capture callback cannot
    // keep moving.
    //
    // `None` until a full interval has been measured. Zero here is an accusation, so the tick
    // that has no previous reading to diff against must not be able to make it.
    pub capture_frames_per_sec: Option<f32>,
    // Audio frames handed to QUIC, counted from `frames_sent` rather than from every
    // datagram this client sends. Position, presence, control and health traffic all leave
    // over the same socket, so the all-traffic rate reads as a healthy microphone on a
    // client that is capturing nothing at all.
    pub datagrams_per_sec: f32,
}
