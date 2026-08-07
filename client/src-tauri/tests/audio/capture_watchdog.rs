use bvc_client_lib::audio::{CaptureVerdict, CaptureWatchdog};

// Three empty reads before acting, which is what the production wiring uses.
const THRESHOLD: u32 = 3;

fn watchdog() -> CaptureWatchdog {
    CaptureWatchdog::new(THRESHOLD)
}

#[test]
fn a_counter_that_advances_is_never_declared_dead() {
    let mut w = watchdog();
    let mut frames = 0u64;

    for _ in 0..50 {
        frames += 50;
        assert_eq!(w.observe(true, frames), CaptureVerdict::Healthy);
    }
}

#[test]
fn the_first_reading_cannot_be_a_verdict() {
    let mut w = watchdog();

    // Nothing to compare against yet. Judging here would restart a stream on the strength of
    // one number, which is what a naive "is it zero" check does to a client that just connected.
    assert_eq!(w.observe(true, 0), CaptureVerdict::Healthy);
}

#[test]
fn a_stalled_counter_is_quiet_until_the_threshold_then_dead() {
    let mut w = watchdog();
    w.observe(true, 100);

    for tick in 1..THRESHOLD {
        assert_eq!(
            w.observe(true, 100),
            CaptureVerdict::Quiet,
            "tick {tick} must not act on its own"
        );
    }

    assert_eq!(w.observe(true, 100), CaptureVerdict::Dead);
}

#[test]
fn one_frame_arriving_before_the_threshold_clears_the_count() {
    let mut w = watchdog();
    w.observe(true, 100);

    assert_eq!(w.observe(true, 100), CaptureVerdict::Quiet);
    assert_eq!(w.observe(true, 101), CaptureVerdict::Healthy);

    // The count restarted rather than resuming: a device that hiccupped and recovered must not
    // be one read away from being torn down for the rest of the session.
    assert_eq!(w.observe(true, 101), CaptureVerdict::Quiet);
    assert_eq!(w.observe(true, 101), CaptureVerdict::Quiet);
    assert_eq!(w.observe(true, 101), CaptureVerdict::Dead);
}

#[test]
fn a_stream_that_is_not_meant_to_be_capturing_is_never_dead() {
    let mut w = watchdog();

    // The setup screen, a disconnected client, a stream between stop and start. Absence is the
    // expected state, and restarting into it would open a microphone nobody asked for.
    for _ in 0..THRESHOLD * 5 {
        assert_eq!(w.observe(false, 0), CaptureVerdict::Healthy);
    }
}

#[test]
fn a_stop_partway_through_a_stall_discards_the_count() {
    let mut w = watchdog();
    w.observe(true, 100);
    assert_eq!(w.observe(true, 100), CaptureVerdict::Quiet);
    assert_eq!(w.observe(true, 100), CaptureVerdict::Quiet);

    // Deliberately stopped, then started again. Carrying the two ticks across would rebuild the
    // new stream on its first read, before it has had a chance to deliver anything.
    assert_eq!(w.observe(false, 100), CaptureVerdict::Healthy);

    assert_eq!(w.observe(true, 100), CaptureVerdict::Healthy);
    assert_eq!(w.observe(true, 100), CaptureVerdict::Quiet);
    assert_eq!(w.observe(true, 100), CaptureVerdict::Quiet);
    assert_eq!(w.observe(true, 100), CaptureVerdict::Dead);
}

#[test]
fn a_counter_reset_under_the_watchdog_is_a_new_baseline_not_a_stall() {
    let mut w = watchdog();
    w.observe(true, 5_000);
    assert_eq!(w.observe(true, 5_000), CaptureVerdict::Quiet);

    // A new session zeroes capture accounting. Read as "fewer frames than last time" it looks
    // like nothing arrived, and the fresh stream is destroyed a read or two after being built.
    assert_eq!(w.observe(true, 0), CaptureVerdict::Healthy);
    assert_eq!(w.observe(true, 20), CaptureVerdict::Healthy);
}

#[test]
fn a_device_that_stays_dead_is_retried_on_a_fixed_cadence() {
    let mut w = watchdog();
    w.observe(true, 100);

    // A device that cannot be reopened must not be retried as fast as the loop runs. Each
    // verdict costs another full threshold, so the restart cadence is the poll interval times
    // the threshold no matter how long the fault lasts.
    for round in 0..4 {
        for _ in 1..THRESHOLD {
            assert_eq!(w.observe(true, 100), CaptureVerdict::Quiet, "round {round}");
        }
        assert_eq!(w.observe(true, 100), CaptureVerdict::Dead, "round {round}");
    }
}
