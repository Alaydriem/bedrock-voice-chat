/// Holds playback until enough packets are queued to absorb jitter, and re-arms after a pause.
///
/// A gate that only ever opens once leaves every utterance after the first starting at zero
/// depth. Re-arming on starvation past the grace window gives each new utterance the same
/// cushion the first one had, and only costs a delay where there was already silence.
#[derive(Debug, Clone)]
pub struct WarmupGate {
    received: usize,
}

impl WarmupGate {
    // The buffer is constructed with its first packet already queued.
    pub fn new() -> Self {
        Self { received: 1 }
    }

    pub fn record_packet(&mut self, needed: usize) {
        self.received = (self.received + 1).min(needed);
    }

    /// Returns true on the one starved frame that crosses the grace window.
    pub fn record_starved_frame(&mut self, starved_frames: u32, grace: u32) -> bool {
        if starved_frames != grace + 1 {
            return false;
        }
        self.received = 0;
        true
    }

    pub fn is_open(&self, needed: usize) -> bool {
        self.received >= needed
    }
}

impl Default for WarmupGate {
    fn default() -> Self {
        Self::new()
    }
}
