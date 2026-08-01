use bvc_client_lib::diagnostics::SampleRing;
use common::structs::metrics::LinkSample;

fn sample(at_ms: u64, rtt_ms: Option<u32>) -> LinkSample {
    LinkSample {
        at_ms,
        rtt_ms,
        uplink_loss_pct: 0.0,
        worst_concealment_pct: 0.0,
    }
}

#[test]
fn ring_evicts_oldest_beyond_capacity() {
    let mut ring = SampleRing::new();
    let overflow = 10u64;

    for i in 0..(SampleRing::CAPACITY as u64 + overflow) {
        ring.push(sample(i, Some(i as u32)));
    }

    assert_eq!(ring.len(), SampleRing::CAPACITY);

    // The first `overflow` entries are gone, so the oldest surviving timestamp is the
    // overflow index rather than zero.
    let samples = ring.samples();
    let oldest = samples.first().expect("ring is not empty");
    assert_eq!(oldest.at_ms, overflow);
}

#[test]
fn percentiles_over_a_known_sequence() {
    let mut ring = SampleRing::new();
    for i in 1..=100u32 {
        ring.push(sample(i as u64, Some(i)));
    }

    assert_eq!(ring.rtt_percentile(50.0), Some(50));
    assert_eq!(ring.rtt_percentile(95.0), Some(95));
    assert_eq!(ring.rtt_max(), Some(100));
}

#[test]
fn percentiles_of_an_empty_ring_are_none() {
    let ring = SampleRing::new();

    // Not Some(0): a zero round trip reads as an impossibly perfect link.
    assert_eq!(ring.rtt_percentile(50.0), None);
    assert_eq!(ring.rtt_percentile(95.0), None);
    assert_eq!(ring.rtt_max(), None);
}

#[test]
fn samples_without_rtt_are_excluded_from_percentiles() {
    let mut ring = SampleRing::new();
    ring.push(sample(1, None));
    ring.push(sample(2, Some(100)));
    ring.push(sample(3, None));

    // A missing measurement must not be counted as 0 ms, which would halve the median.
    assert_eq!(ring.rtt_percentile(50.0), Some(100));
    assert_eq!(ring.rtt_max(), Some(100));
}

#[test]
fn the_percentile_extremes_are_the_min_and_the_max() {
    let mut ring = SampleRing::new();
    for v in [10u32, 20, 30, 40] {
        ring.push(sample(v as u64, Some(v)));
    }

    // Nearest-rank, so p50 of four samples is the second: an index computed from a float must not
    // drift off either end.
    assert_eq!(ring.rtt_percentile(0.0), Some(10));
    assert_eq!(ring.rtt_percentile(25.0), Some(10));
    assert_eq!(ring.rtt_percentile(50.0), Some(20));
    assert_eq!(ring.rtt_percentile(100.0), Some(40));
    assert_eq!(ring.rtt_percentile(100.0), ring.rtt_max());
}

#[test]
fn an_out_of_range_percentile_clamps_rather_than_panicking() {
    let mut ring = SampleRing::new();
    for v in [10u32, 20, 30] {
        ring.push(sample(v as u64, Some(v)));
    }

    assert_eq!(ring.rtt_percentile(150.0), Some(30));
    assert_eq!(ring.rtt_percentile(-10.0), Some(10));
}

#[test]
fn a_ring_of_only_missing_rtt_yields_none() {
    let mut ring = SampleRing::new();
    ring.push(sample(1, None));
    ring.push(sample(2, None));

    assert_eq!(ring.len(), 2);
    assert_eq!(ring.rtt_percentile(50.0), None);
}
