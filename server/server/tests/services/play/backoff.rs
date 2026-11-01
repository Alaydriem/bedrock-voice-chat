use std::time::Duration;

use bvc_server_lib::services::play::PlayBackoff;

// Boot waits 5 s, 10 s, 20 s and so on, and stops growing at the cap.
#[test]
fn the_wait_doubles_until_it_reaches_the_cap() {
    let mut backoff = PlayBackoff::new(Duration::from_secs(5), Duration::from_secs(300));

    let waits: Vec<u64> = (0..9).map(|_| backoff.next().as_secs()).collect();

    assert_eq!(waits, vec![5, 10, 20, 40, 80, 160, 300, 300, 300]);
}

// Renewal waits 60 s doubling to an hour.
#[test]
fn the_renewal_schedule_ends_at_one_hour() {
    let mut backoff = PlayBackoff::new(Duration::from_secs(60), Duration::from_secs(3600));

    let last = (0..20).map(|_| backoff.next()).last().unwrap();

    assert_eq!(last, Duration::from_secs(3600));
}

// An initial wait above the cap is a configuration slip, and must not exceed the cap.
#[test]
fn an_initial_wait_above_the_cap_is_clamped() {
    let mut backoff = PlayBackoff::new(Duration::from_secs(900), Duration::from_secs(300));

    assert_eq!(backoff.next(), Duration::from_secs(300));
}
