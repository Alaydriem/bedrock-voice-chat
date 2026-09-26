use bvc_client_lib::DrainPolicy;

const TARGET: usize = 3;
const QUIET: f32 = 0.0;
const LOUD: f32 = 0.5;

#[test]
fn a_backlog_inside_the_hysteresis_is_never_shed() {
    let mut policy = DrainPolicy::new();
    let depth = TARGET + DrainPolicy::HYSTERESIS_FRAMES;
    assert!(!policy.should_shed(depth, TARGET, QUIET));
}

#[test]
fn a_quiet_excess_sheds() {
    let mut policy = DrainPolicy::new();
    let depth = TARGET + DrainPolicy::HYSTERESIS_FRAMES + 1;
    assert!(policy.should_shed(depth, TARGET, QUIET));
}

#[test]
fn a_loud_excess_under_the_forced_bound_does_not_shed() {
    let mut policy = DrainPolicy::new();
    let depth = TARGET + DrainPolicy::FORCED_EXCESS_FRAMES;
    assert!(!policy.should_shed(depth, TARGET, LOUD));
}

#[test]
fn a_loud_excess_past_the_forced_bound_sheds() {
    let mut policy = DrainPolicy::new();
    let depth = TARGET + DrainPolicy::FORCED_EXCESS_FRAMES + 1;
    assert!(policy.should_shed(depth, TARGET, LOUD));
}

// One shed per spacing window below the flush bound, however persistent the excess.
#[test]
fn sheds_are_spaced() {
    let mut policy = DrainPolicy::new();
    let deep = TARGET + DrainPolicy::FORCED_EXCESS_FRAMES + 5;
    assert!(policy.should_shed(deep, TARGET, QUIET));
    for _ in 1..DrainPolicy::SHED_SPACING_FRAMES {
        assert!(!policy.should_shed(deep, TARGET, QUIET));
    }
    assert!(policy.should_shed(deep, TARGET, QUIET));
}

// Past the flush bound, every frame sheds, ignoring spacing and loudness, until depth is back at
// the target plus hysteresis. Then it stops, and the ordinary rules resume.
#[test]
fn a_runaway_backlog_flushes_to_target_plus_hysteresis() {
    let mut policy = DrainPolicy::new();
    let start = TARGET + DrainPolicy::FLUSH_EXCESS_FRAMES + 5;
    let floor = TARGET + DrainPolicy::HYSTERESIS_FRAMES;

    for depth in (floor + 1..=start).rev() {
        assert!(
            policy.should_shed(depth, TARGET, LOUD),
            "depth {depth} should flush"
        );
    }
    assert!(!policy.should_shed(floor, TARGET, LOUD));
    assert!(!policy.should_shed(floor + 2, TARGET, LOUD));
}
