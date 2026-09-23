/// Decides, frame by frame, whether to shed the frame at the front of the ring instead of playing it.
///
/// Admitting every frame means a burst of late arrivals leaves the ring deeper than before, and
/// depth is latency. Shedding brings it back toward the warmup target. The candidate is judged by
/// its own loudness, so a shed falls where nothing is audible, unless the excess is so large that
/// a continuous talker would otherwise never be brought back. A backlog past the flush bound is
/// cleared in one contiguous run rather than shed one frame per window for seconds.
#[derive(Debug, Clone)]
pub struct DrainPolicy {
    frames_since_shed: u32,
    flushing: bool,
}

impl DrainPolicy {
    // Depth this far above target is left alone: ordinary jitter, not a backlog.
    pub const HYSTERESIS_FRAMES: usize = 3;
    // 200 ms of excess sheds even through speech.
    pub const FORCED_EXCESS_FRAMES: usize = 10;
    // 500 ms of excess is a stall, not jitter: cleared in one run.
    pub const FLUSH_EXCESS_FRAMES: usize = 25;
    // At most one shed per 100 ms of playback outside a flush, so no two skips land close together.
    pub const SHED_SPACING_FRAMES: u32 = 5;
    // About -40 dBFS.
    pub const QUIET_RMS: f32 = 0.01;

    pub fn new() -> Self {
        Self {
            frames_since_shed: Self::SHED_SPACING_FRAMES,
            flushing: false,
        }
    }

    pub fn should_shed(&mut self, depth: usize, target: usize, candidate_rms: f32) -> bool {
        self.frames_since_shed = self.frames_since_shed.saturating_add(1);
        let excess = depth.saturating_sub(target);

        if excess > Self::FLUSH_EXCESS_FRAMES {
            self.flushing = true;
        }
        if self.flushing {
            if excess > Self::HYSTERESIS_FRAMES {
                self.frames_since_shed = 0;
                return true;
            }
            self.flushing = false;
            return false;
        }

        let quiet = candidate_rms < Self::QUIET_RMS;
        let shed = self.frames_since_shed >= Self::SHED_SPACING_FRAMES
            && excess > Self::HYSTERESIS_FRAMES
            && (quiet || excess > Self::FORCED_EXCESS_FRAMES);

        if shed {
            self.frames_since_shed = 0;
        }
        shed
    }
}

impl Default for DrainPolicy {
    fn default() -> Self {
        Self::new()
    }
}
