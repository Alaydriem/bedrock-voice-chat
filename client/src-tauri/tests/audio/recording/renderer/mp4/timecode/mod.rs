mod sample;

use bvc_client_lib::audio::recording::renderer::mp4::timecode::Timecode;

#[test]
fn the_frame_rate_is_fifty_per_second() {
    assert_eq!(Timecode::FPS, 50);
}

#[test]
fn the_frame_index_tracks_the_millisecond_offset() {
    // 500ms into a second is frame 25 at 50fps
    let tc = Timecode::new(1000 * 60 * 60 + 500, 48000);
    assert_eq!(tc.frames(), 25);

    let tc = Timecode::new(1000 * 60 * 60, 48000);
    assert_eq!(tc.frames(), 0);

    let tc = Timecode::new(1000 * 60 * 60 + 980, 48000);
    assert_eq!(tc.frames(), 49);
}

#[test]
fn a_frame_is_twenty_milliseconds_of_samples() {
    let tc = Timecode::new(0, 48000);
    assert_eq!(tc.frame_duration_samples(), 960);
}

#[test]
fn the_frame_number_stays_within_a_day() {
    // The h:m:s components depend on the local timezone, so the formula is
    // bounded rather than asserted against a fixed value.
    let tc = Timecode::new(0, 48000);
    assert!(tc.to_frame_number() < 24 * 3600 * 50);
}

#[test]
fn the_frame_index_never_leaves_its_bounds() {
    for ms in (0..1000).step_by(20) {
        let tc = Timecode::new(ms, 48000);
        assert!(
            tc.frames() < 50,
            "Frame {} out of bounds for {}ms",
            tc.frames(),
            ms
        );
    }
}
