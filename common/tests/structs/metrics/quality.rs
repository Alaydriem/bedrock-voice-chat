use common::structs::metrics::{
    LOSS_BAD_PCT, LOSS_DEGRADED_PCT, LinkQuality, RTT_BAD_MS, RTT_DEGRADED_MS,
};

// Threshold values are provisional and set by the operator from field data, so these
// reference the constants rather than literals: changing a number must not break a test,
// and a test must not silently enshrine a guess. What is pinned is the mechanism —
// per-dimension severity, worst dimension wins, boundaries inclusive.

#[test]
fn below_every_threshold_is_good() {
    assert_eq!(LinkQuality::classify(0.0, 0), LinkQuality::Good);
}

#[test]
fn the_degraded_loss_boundary_is_inclusive() {
    assert_eq!(
        LinkQuality::classify(LOSS_DEGRADED_PCT, 0),
        LinkQuality::Degraded
    );
}

#[test]
fn the_bad_loss_boundary_is_inclusive() {
    assert_eq!(LinkQuality::classify(LOSS_BAD_PCT, 0), LinkQuality::Bad);
}

#[test]
fn rtt_alone_degrades_a_lossless_link() {
    assert_eq!(
        LinkQuality::classify(0.0, RTT_DEGRADED_MS),
        LinkQuality::Degraded
    );
}

#[test]
fn worst_dimension_wins() {
    assert_eq!(LinkQuality::classify(LOSS_BAD_PCT, 0), LinkQuality::Bad);
    assert_eq!(LinkQuality::classify(0.0, RTT_BAD_MS), LinkQuality::Bad);
}

#[test]
fn thresholds_are_ordered() {
    // A misordered pair would make one tier unreachable.
    assert!(LOSS_DEGRADED_PCT < LOSS_BAD_PCT);
    assert!(RTT_DEGRADED_MS < RTT_BAD_MS);
}
