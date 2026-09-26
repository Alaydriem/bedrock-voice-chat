use super::rig::JitterBufferRig;

// A short consecutive loss must cost exactly the frames that were lost. Frames 10 and 11 never
// arrive; everything after them did and must be decoded.
#[test]
fn a_two_frame_loss_costs_only_the_two_frames() {
    let mut rig = JitterBufferRig::new(41, JitterBufferRig::LOUD, false);

    rig.stream(1..41, &[10, 11], 10);

    assert_eq!(
        rig.stats().ooo_drops(),
        0,
        "frames after the gap were rejected as out of order"
    );
    // Frames 0–9 and 12–40.
    assert_eq!(rig.stats().frames_decoded(), 39);
    assert_eq!(
        rig.stats().gap_frames(),
        2,
        "the two lost frames are counted as the gap"
    );
    rig.assert_initial_config();
}

// After a pause, the first frame back must wait for depth to rebuild rather than play at zero
// depth. Frames 0–9 are an utterance; the twenty empty frames are the speaker's pause; frame 55
// onward is the next utterance.
#[test]
fn the_first_frame_after_a_pause_waits_for_warmup() {
    let mut rig = JitterBufferRig::new(70, JitterBufferRig::LOUD, false);
    let needed = rig.stats().warmup_needed() as usize;

    rig.stream(1..10, &[], 0);
    rig.pull_frames(20);
    assert_eq!(rig.stats().frames_decoded(), 10);

    rig.send(55);
    rig.pull_frames(1);
    assert_eq!(
        rig.stats().frames_decoded(),
        10,
        "the first frame after the pause played at zero depth instead of waiting"
    );

    rig.send_range(56..55 + needed);
    rig.pull_frames(needed + 2);
    assert_eq!(rig.stats().frames_decoded(), 10 + needed as u64);
    assert_eq!(rig.stats().warmup_rearms(), 1);
    rig.assert_initial_config();
}

// A frame the listener never heard must not be in the recording. Frame 5 arrives but cannot be
// decoded, so playback conceals it; the recording must hold every other frame, in order, and
// not frame 5's payload.
#[test]
fn a_frame_that_fails_to_decode_is_not_recorded() {
    let mut rig = JitterBufferRig::new(11, JitterBufferRig::LOUD, true);
    // A code-3 Opus packet with no frame-count byte: libopus rejects it as invalid.
    let undecodable = bytes::Bytes::from_static(&[0x03]);

    for index in 1..11 {
        if index == 5 {
            rig.send_payload(5, undecodable.clone());
        } else {
            rig.send(index);
        }
        rig.pull_frames(1);
    }
    rig.pull_frames(10);

    let expected: Vec<Vec<u8>> = (0..11)
        .filter(|index| *index != 5)
        .map(|index| rig.payload(index))
        .collect();

    assert_eq!(rig.recorded_payloads(), expected);
}

// A quiet backlog is drained by the quiet rule alone. 13 frames against a target of 3 is an excess
// of 10, which is not past the forced bound, so nothing here can be a forced shed: this test
// fails if the quiet branch is dead.
#[test]
fn a_quiet_backlog_is_drained_by_the_quiet_rule_alone() {
    let mut rig = JitterBufferRig::new(13, JitterBufferRig::QUIET, false);

    rig.send_range(1..13);
    rig.pull_frames(25);

    let decoded = rig.stats().frames_decoded();
    let shed = rig.stats().drain_sheds();
    assert!(
        shed >= 1,
        "a quiet backlog past the hysteresis must be drained; shed {shed}"
    );
    assert_eq!(
        decoded + shed,
        13,
        "decoded {decoded} + shed {shed} must account for every frame"
    );
    assert_eq!(rig.stats().ooo_drops(), 0);
    assert_eq!(rig.stats().overflow_drops(), 0);
    rig.assert_initial_config();
}

// Loud speech with a modest backlog is never cut. 12 frames is an excess of 9 against a target of
// 3 (and 10 against the production target of 2): past the hysteresis, not past the forced bound.
#[test]
fn loud_speech_under_the_forced_bound_is_never_shed() {
    let mut rig = JitterBufferRig::new(12, JitterBufferRig::LOUD, false);

    rig.send_range(1..12);
    rig.pull_frames(25);

    assert_eq!(rig.stats().drain_sheds(), 0);
    assert_eq!(rig.stats().frames_decoded(), 12);
    rig.assert_initial_config();
}

// Past the forced bound, loud frames shed too, one per spacing window, and stop once the excess is
// back under the bound. From 20 queued: shed at depth 20, play five, shed at depth 15, then the
// excess is 7 and loud frames play.
#[test]
fn a_loud_backlog_past_the_forced_bound_is_shed_in_spaced_steps() {
    let mut rig = JitterBufferRig::new(20, JitterBufferRig::LOUD, false);

    rig.send_range(1..20);
    rig.pull_frames(30);

    assert_eq!(rig.stats().drain_sheds(), 2);
    assert_eq!(rig.stats().frames_decoded(), 18);
    rig.assert_initial_config();
}

// A stalled device leaves a backlog past the flush bound. It is flushed in one step, down to the
// target plus hysteresis, instead of being shed one frame per window for seconds.
#[test]
fn a_runaway_backlog_is_flushed_in_one_step() {
    let mut rig = JitterBufferRig::new(41, JitterBufferRig::LOUD, false);

    rig.send_range(1..41);
    rig.pull_frames(1);

    // Depth 41 falls to 6 (target 3 + hysteresis 3) before the first frame plays.
    assert_eq!(rig.stats().drain_sheds(), 35);

    rig.pull_frames(20);
    assert_eq!(
        rig.stats().frames_decoded() + rig.stats().drain_sheds(),
        41
    );
    rig.assert_initial_config();
}
