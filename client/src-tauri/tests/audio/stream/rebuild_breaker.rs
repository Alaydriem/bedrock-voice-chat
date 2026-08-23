use bvc_client_lib::audio::{RebuildBreaker, RebuildVerdict};
use common::structs::audio::AudioDeviceType;
use std::time::Duration;

fn retry_after(verdict: RebuildVerdict) -> Duration {
    match verdict {
        RebuildVerdict::Retry { after, .. } => after,
        RebuildVerdict::Open => panic!("expected a retry, the breaker opened"),
    }
}

const INPUT: AudioDeviceType = AudioDeviceType::InputDevice;
const OUTPUT: AudioDeviceType = AudioDeviceType::OutputDevice;

#[test]
fn each_retry_waits_twice_as_long_as_the_last() {
    let mut breaker = RebuildBreaker::new();

    let delays: Vec<Duration> = (0..5)
        .map(|_| retry_after(breaker.observe_failure(&INPUT)))
        .collect();

    assert_eq!(
        delays,
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(8),
            Duration::from_secs(16),
        ]
    );
}

#[test]
fn the_sixth_failure_opens_the_breaker() {
    let mut breaker = RebuildBreaker::new();

    for _ in 0..5 {
        breaker.observe_failure(&INPUT);
    }

    assert_eq!(breaker.observe_failure(&INPUT), RebuildVerdict::Open);
    assert!(breaker.is_open(&INPUT));
}

// An open breaker that kept answering `Open` would be harmless; one that started retrying
// again would resume the loop this whole change exists to stop.
#[test]
fn an_open_breaker_stays_open() {
    let mut breaker = RebuildBreaker::new();

    for _ in 0..8 {
        breaker.observe_failure(&INPUT);
    }

    assert_eq!(breaker.observe_failure(&INPUT), RebuildVerdict::Open);
}

#[test]
fn a_success_returns_the_next_failure_to_the_shortest_delay() {
    let mut breaker = RebuildBreaker::new();
    breaker.observe_failure(&INPUT);
    breaker.observe_failure(&INPUT);

    breaker.observe_success(&INPUT);

    assert_eq!(
        retry_after(breaker.observe_failure(&INPUT)),
        Duration::from_secs(1)
    );
}

#[test]
fn rearm_clears_an_open_breaker() {
    let mut breaker = RebuildBreaker::new();
    for _ in 0..6 {
        breaker.observe_failure(&INPUT);
    }
    assert!(breaker.is_open(&INPUT));

    breaker.rearm(&INPUT);

    assert!(!breaker.is_open(&INPUT));
    assert_eq!(
        retry_after(breaker.observe_failure(&INPUT)),
        Duration::from_secs(1)
    );
}

// A broken microphone must not spend the speakers' retry budget. Sharing one count would
// make a failing input open the breaker for an output that had never failed.
#[test]
fn the_two_devices_do_not_share_a_count() {
    let mut breaker = RebuildBreaker::new();

    for _ in 0..6 {
        breaker.observe_failure(&INPUT);
    }

    assert!(breaker.is_open(&INPUT));
    assert!(!breaker.is_open(&OUTPUT));
    assert_eq!(
        retry_after(breaker.observe_failure(&OUTPUT)),
        Duration::from_secs(1)
    );
}
