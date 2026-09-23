mod frame_admission;

pub use frame_admission::FrameAdmission;

/// What the jitter buffer does with an arriving frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    // Ahead of the last admitted frame and inside the band. `missing_frames` is how many whole
    // frames the gap skipped: loss, or a speaker pause shorter than the band.
    Accept { missing_frames: u64 },
    // At or behind the last admitted frame: a duplicate, or a late arrival the arrival-ordered
    // ring would otherwise play out of order.
    Reject,
    // Ahead by more than the band: a speaker resuming after a pause. Played like an accept.
    Resume,
    // Behind by more than the band: the sender's clock restarted. Played like an accept.
    Reanchor,
}
