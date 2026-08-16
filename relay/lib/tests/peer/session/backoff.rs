use bvc_relay::peer::session::Backoff;

// The first retry is quick because the common cause is a server restart, which
// is over in seconds. Waiting the ceiling for that would turn a blip into half a
// minute of silence.
#[test]
fn the_first_retry_is_quick() {
    let mut backoff = Backoff::new();

    assert_eq!(backoff.next_delay(), Backoff::FIRST);
}

#[test]
fn successive_retries_grow() {
    let mut backoff = Backoff::new();
    let first = backoff.next_delay();
    let second = backoff.next_delay();

    assert!(second > first, "{second:?} did not grow from {first:?}");
}

// A peer down for an hour must not be dialled at an interval that keeps growing:
// the ceiling is what stops a long outage becoming a slow scan, and what stops
// the shift arithmetic overflowing into nonsense.
#[test]
fn growth_stops_at_the_ceiling() {
    let mut backoff = Backoff::new();

    for _ in 0..64 {
        assert!(
            backoff.next_delay() <= Backoff::CEILING,
            "a delay exceeded the ceiling"
        );
    }

    assert_eq!(backoff.next_delay(), Backoff::CEILING);
}

// A link that connected and then dropped an hour later is a fresh outage, not a
// continuation of the last one. Without the reset it would retry at the ceiling
// immediately, which is the slowest possible response to the most recoverable
// kind of failure.
#[test]
fn a_successful_connect_resets_the_schedule() {
    let mut backoff = Backoff::new();
    for _ in 0..10 {
        backoff.next_delay();
    }

    backoff.reset();

    assert_eq!(backoff.next_delay(), Backoff::FIRST);
}
