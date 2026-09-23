use super::rig::JitterBufferRig;

// Everything below this test asserts that a specific input loses nothing. This one proves the
// rig itself loses nothing on a clean, contiguous stream, so a later failure is the buffer's.
#[test]
fn a_contiguous_stream_is_decoded_frame_for_frame() {
    let mut rig = JitterBufferRig::new(30, JitterBufferRig::LOUD, false);

    rig.stream(1..30, &[], 10);

    assert_eq!(rig.stats().frames_decoded(), 30);
    assert_eq!(rig.stats().ooo_drops(), 0);
    assert_eq!(rig.stats().overflow_drops(), 0);
}
