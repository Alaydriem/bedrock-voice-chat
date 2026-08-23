use bvc_client_lib::audio::spatial::{GainSmoother, SpatialGains};

fn target() -> SpatialGains {
    SpatialGains::from_pan(1.0, 0.5, 1.0)
}

// One step moves toward the target and never past it. A smoother that overshoots would make a
// voice audibly wobble every time somebody turned.
#[test]
fn one_advance_moves_toward_the_target_without_overshooting() {
    let start = SpatialGains::centred();
    let goal = target();
    let mut smoother = GainSmoother::new(start);

    let stepped = smoother.advance(&goal);

    assert!(stepped.left > start.left);
    assert!(stepped.left < goal.left);
    assert!(stepped.volume < start.volume);
    assert!(stepped.volume > goal.volume);
}

// Past the settle count the remaining error is below anything audible, which is what makes
// skipping a long gap equivalent to walking it.
//
// The bound is 1e-4, roughly -80dB, rather than something tighter: the ramp is f32, so once the
// step falls below half an ulp of the running value the addition rounds to nothing and the walk
// stops short of the target by a margin no assertion can close. Snapping is what reaches it
// exactly, which is the reason `advance_by` exists.
#[test]
fn the_target_is_reached_within_the_settle_count() {
    let goal = target();
    let mut smoother = GainSmoother::new(SpatialGains::centred());

    let mut last = SpatialGains::centred();
    for _ in 0..GainSmoother::SETTLE_SAMPLES {
        last = smoother.advance(&goal);
    }

    assert!((last.left - goal.left).abs() < 1e-4, "left {}", last.left);
    assert!((last.right - goal.right).abs() < 1e-4, "right {}", last.right);
    assert!(
        (last.volume - goal.volume).abs() < 1e-4,
        "volume {}",
        last.volume
    );
}

#[test]
fn advance_by_agrees_with_the_same_number_of_single_advances() {
    let goal = target();
    let mut walked = GainSmoother::new(SpatialGains::centred());
    let mut skipped = GainSmoother::new(SpatialGains::centred());

    let mut last = SpatialGains::centred();
    for _ in 0..600 {
        last = walked.advance(&goal);
    }
    skipped.advance_by(&goal, 600);
    let after_skip = skipped.advance(&goal);
    let after_walk = walked.advance(&goal);

    assert!((after_skip.left - after_walk.left).abs() < 1e-5);
    assert!((after_skip.volume - after_walk.volume).abs() < 1e-5);
    assert!(last.left < goal.left);
}

// A gap longer than the settle count is not walked sample by sample, and the result has to be
// the settled target rather than something short of it.
#[test]
fn a_gap_longer_than_the_settle_count_lands_on_the_target() {
    let goal = target();
    let mut smoother = GainSmoother::new(SpatialGains::centred());

    smoother.advance_by(&goal, GainSmoother::SETTLE_SAMPLES * 4);
    let after = smoother.advance(&goal);

    assert!((after.left - goal.left).abs() < 1e-6);
    assert!((after.volume - goal.volume).abs() < 1e-6);
}

#[test]
fn advancing_by_zero_samples_changes_nothing() {
    let goal = target();
    let mut smoother = GainSmoother::new(SpatialGains::centred());

    smoother.advance_by(&goal, 0);
    let after = smoother.advance(&goal);
    let mut reference = GainSmoother::new(SpatialGains::centred());
    let expected = reference.advance(&goal);

    assert!((after.left - expected.left).abs() < 1e-9);
}

// A smoother already sitting on its target stays there rather than drifting.
#[test]
fn a_smoother_at_its_target_holds_still() {
    let goal = target();
    let mut smoother = GainSmoother::new(goal);

    let after = smoother.advance(&goal);

    assert!((after.left - goal.left).abs() < 1e-9);
    assert!((after.right - goal.right).abs() < 1e-9);
    assert!((after.volume - goal.volume).abs() < 1e-9);
}
