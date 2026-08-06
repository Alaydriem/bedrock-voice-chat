use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct NoiseGateSettings {
    pub open_threshold: f32,
    pub close_threshold: f32,
    pub release_rate: f32,
    pub attack_rate: f32,
    pub hold_time: f32,
}

impl Default for NoiseGateSettings {
    /// The same five numbers the client's own defaults carry, so a gate the backend fell
    /// back to and a gate the user reset are the same gate. They diverged once, and the
    /// symptom was a microphone that passed nothing until the user pressed Reset.
    fn default() -> Self {
        Self {
            open_threshold: -40.0,
            close_threshold: -50.0,
            release_rate: 100.0,
            attack_rate: 10.0,
            hold_time: 50.0,
        }
    }
}

impl NoiseGateSettings {
    /// Whether these numbers describe a gate that can actually shut.
    ///
    /// A close threshold at or above the open threshold latches the gate open on the
    /// first sound and never releases it. Nothing in the audio path refuses that, so it
    /// reaches the user as the noise gate silently not working.
    pub fn can_close(&self) -> bool {
        self.close_threshold < self.open_threshold
    }
}
