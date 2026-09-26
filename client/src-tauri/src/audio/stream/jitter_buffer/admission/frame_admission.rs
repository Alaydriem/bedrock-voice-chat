use super::Admission;

/// Decides admission from a frame's sender timestamp alone.
///
/// A gap ahead of the last frame is loss, never reordering, so it is always admitted. The only
/// rejection is a frame at or behind the last one. A jump wider than the band in either direction
/// is a new timeline, which is what keeps a restarted sender clock or one far-future stamp from
/// leaving every following frame permanently "behind".
#[derive(Debug, Clone)]
pub struct FrameAdmission {
    last: u64,
}

impl FrameAdmission {
    // Wide enough that no loss burst worth reasoning about crosses it; the same one second the
    // old rule used as its escape hatch.
    pub const REANCHOR_BAND_MS: u64 = 1_000;

    pub fn new(first: u64) -> Self {
        Self { last: first }
    }

    pub fn admit(&mut self, timestamp: u64) -> Admission {
        if timestamp > self.last {
            let gap = timestamp - self.last;
            self.last = timestamp;
            if gap > Self::REANCHOR_BAND_MS {
                return Admission::Resume;
            }
            return Admission::Accept {
                missing_frames: Self::missing_frames(gap),
            };
        }

        if self.last - timestamp > Self::REANCHOR_BAND_MS {
            self.last = timestamp;
            return Admission::Reanchor;
        }

        Admission::Reject
    }

    pub fn last(&self) -> u64 {
        self.last
    }

    // Rounded to whole frames, because wall-clock stamps land a millisecond or two either side of
    // the 20 ms grid. A gap of one frame is contiguous and misses nothing.
    fn missing_frames(gap: u64) -> u64 {
        let frame = common::consts::OPUS_FRAME_DURATION_MS as u64;
        ((gap + frame / 2) / frame).saturating_sub(1)
    }
}
