use bvc_client_lib::{Admission, FrameAdmission};

const FRAME: u64 = 20;

fn accept(missing_frames: u64) -> Admission {
    Admission::Accept { missing_frames }
}

#[test]
fn contiguous_frames_are_accepted_with_nothing_missing() {
    let mut admission = FrameAdmission::new(0);
    assert_eq!(admission.admit(FRAME), accept(0));
    assert_eq!(admission.admit(2 * FRAME), accept(0));
}

// The case the old 40 ms band rejected, along with everything after it.
#[test]
fn a_gap_inside_the_band_is_accepted_and_so_is_what_follows() {
    let mut admission = FrameAdmission::new(0);
    assert_eq!(admission.admit(3 * FRAME), accept(2));
    assert_eq!(admission.admit(4 * FRAME), accept(0));
}

// Stamps are wall-clock milliseconds, so a one-frame loss arrives as 39–41 ms, not exactly 40.
#[test]
fn the_missing_count_rounds_to_whole_frames() {
    let mut admission = FrameAdmission::new(0);
    assert_eq!(admission.admit(41), accept(1));
    assert_eq!(admission.admit(41 + 21), accept(0));
}

#[test]
fn a_gap_of_most_of_the_band_is_accepted() {
    let mut admission = FrameAdmission::new(0);
    assert_eq!(admission.admit(600), accept(29));
}

#[test]
fn a_duplicate_is_rejected() {
    let mut admission = FrameAdmission::new(100);
    assert_eq!(admission.admit(100), Admission::Reject);
}

// The ring is arrival-ordered, so admitting this would play it after a later frame.
#[test]
fn a_late_frame_inside_the_band_is_rejected() {
    let mut admission = FrameAdmission::new(200);
    assert_eq!(admission.admit(180), Admission::Reject);
    assert_eq!(admission.last(), 200);
}

// A speaker coming back from a pause longer than the band. Not loss, and not a clock fault.
#[test]
fn a_forward_jump_past_the_band_resumes() {
    let mut admission = FrameAdmission::new(0);
    let far = FrameAdmission::REANCHOR_BAND_MS + FRAME;
    assert_eq!(admission.admit(far), Admission::Resume);
    assert_eq!(admission.admit(far + FRAME), accept(0));
}

// A sender whose clock restarted, or a single far-future stamp, must not mute that sender for
// the rest of the session. Without this, every later frame would be "behind" and rejected.
#[test]
fn a_backward_jump_past_the_band_reanchors_and_the_new_timeline_is_accepted() {
    let mut admission = FrameAdmission::new(10_000_000);
    assert_eq!(admission.admit(40), Admission::Reanchor);
    assert_eq!(admission.admit(60), accept(0));
    assert_eq!(admission.admit(80), accept(0));
}
