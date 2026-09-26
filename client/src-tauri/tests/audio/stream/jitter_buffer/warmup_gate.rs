use bvc_client_lib::WarmupGate;

const NEEDED: usize = 3;
const GRACE: u32 = 5;

// The construction packet counts, so two more open the gate.
#[test]
fn the_construction_packet_counts_toward_warmup() {
    let mut gate = WarmupGate::new();
    assert!(!gate.is_open(NEEDED));
    gate.record_packet(NEEDED);
    gate.record_packet(NEEDED);
    assert!(gate.is_open(NEEDED));
}

// Jitter the grace window covers must not cost a warmup.
#[test]
fn starvation_inside_the_grace_window_does_not_rearm() {
    let mut gate = WarmupGate::new();
    gate.record_packet(NEEDED);
    gate.record_packet(NEEDED);
    for starved in 1..=GRACE {
        assert!(!gate.record_starved_frame(starved, GRACE));
    }
    assert!(gate.is_open(NEEDED));
}

// Once per gap: the frame that crosses the window re-arms, the ones after it do not count again.
#[test]
fn starvation_past_the_grace_window_rearms_once() {
    let mut gate = WarmupGate::new();
    gate.record_packet(NEEDED);
    gate.record_packet(NEEDED);

    let rearms = (1..=GRACE + 20)
        .filter(|starved| gate.record_starved_frame(*starved, GRACE))
        .count();

    assert_eq!(rearms, 1);
    assert!(!gate.is_open(NEEDED));
    for _ in 0..NEEDED {
        gate.record_packet(NEEDED);
    }
    assert!(gate.is_open(NEEDED));
}
