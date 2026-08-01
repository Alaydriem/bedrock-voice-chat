use std::sync::Arc;
use std::time::Duration;

use bvc_client_lib::diagnostics::{QuicLinkStats, QuicStatsSubscriber};
use tokio::sync::watch;

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[test]
fn recovery_metrics_on_the_active_path_records_rtt() {
    let stats = QuicLinkStats::new();
    stats.record_rtt_for_path(true, ms(89), ms(92), ms(80), ms(4));

    // Only the smoothed value and the variance are published; the others exist so a future reader
    // is not tempted to re-derive them from the wrong field.
    assert_eq!(stats.smoothed_rtt_ms(), Some(89));
    assert_eq!(stats.rtt_variance_ms(), Some(4));
}

#[test]
fn recovery_metrics_on_an_inactive_path_does_not_overwrite_rtt() {
    let stats = QuicLinkStats::new();
    stats.record_rtt_for_path(true, ms(89), ms(92), ms(80), ms(4));

    // A measurement for a path that is no longer carrying traffic must not displace the live
    // one, or a migrated connection reports a dead path's round trip.
    stats.record_rtt_for_path(false, ms(900), ms(950), ms(880), ms(70));

    assert_eq!(stats.smoothed_rtt_ms(), Some(89));
    assert_eq!(stats.latest_rtt_ms(), Some(92));
}

#[test]
fn an_inactive_path_alone_leaves_rtt_unmeasured() {
    let stats = QuicLinkStats::new();
    stats.record_rtt_for_path(false, ms(89), ms(92), ms(80), ms(4));

    // Not Some(0) — nothing has been measured on a live path yet.
    assert_eq!(stats.smoothed_rtt_ms(), None);
}

#[test]
fn rtt_is_unmeasured_until_a_sample_arrives() {
    let stats = QuicLinkStats::new();

    assert_eq!(stats.smoothed_rtt_ms(), None);
    assert_eq!(stats.min_rtt_ms(), None);
    assert_eq!(stats.rtt_variance_ms(), None);
}

#[test]
fn a_genuine_sub_millisecond_rtt_is_measured_not_absent() {
    let stats = QuicLinkStats::new();
    stats.record_rtt_for_path(
        true,
        Duration::from_micros(400),
        Duration::from_micros(400),
        Duration::from_micros(400),
        Duration::from_micros(10),
    );

    // Truncates to 0 ms, but it is a real measurement: Some(0), never None.
    assert_eq!(stats.smoothed_rtt_ms(), Some(0));
}

#[test]
fn each_connection_context_is_a_fresh_stats_handle() {
    let (tx, mut rx) = watch::channel(Arc::new(QuicLinkStats::new()));
    let subscriber = QuicStatsSubscriber::new(tx);

    let first = subscriber.mint_context();
    first.record_sent();
    first.record_lost();
    assert_eq!(rx.borrow_and_update().packets_sent(), 1);

    // A reconnect must start from zero rather than inherit a dead connection's totals.
    let second = subscriber.mint_context();

    assert_eq!(second.packets_sent(), 0);
    assert_eq!(second.packets_lost(), 0);
    assert!(!Arc::ptr_eq(&first, &second));

    // ...and the watch now publishes the new handle, so a reader follows the live connection.
    let published = rx.borrow_and_update().clone();
    assert!(Arc::ptr_eq(&published, &second));
}

// Downlink loss from QUIC packet numbers.
//
// The peer deliberately skips single packet numbers roughly once per congestion window, and the skip
// event fires on the sender — so a receiver cannot identify them. A skip is always exactly one
// number, which is what makes runs of two or more provable loss and isolated gaps ambiguous.

#[test]
fn a_single_missing_number_is_not_counted_as_burst_loss() {
    let stats = QuicLinkStats::new();
    stats.record_packet_number(10);
    stats.record_packet_number(12);

    // 11 is missing. That is either a deliberate skip or one real loss, and counting it would put a
    // 0.7-2.9% phantom floor on every healthy link — past the 1% Degraded threshold.
    assert_eq!(stats.burst_loss(), 0);
    assert_eq!(stats.isolated_gaps(), 1);
}

#[test]
fn a_run_of_two_missing_numbers_counts_as_two_lost() {
    let stats = QuicLinkStats::new();
    stats.record_packet_number(10);
    stats.record_packet_number(13);

    // 11 and 12. No skip mechanism produces two consecutive, so this is loss.
    assert_eq!(stats.burst_loss(), 2);
    assert_eq!(stats.isolated_gaps(), 0);
}

#[test]
fn a_long_run_counts_every_missing_number() {
    let stats = QuicLinkStats::new();
    stats.record_packet_number(100);
    stats.record_packet_number(150);

    assert_eq!(stats.burst_loss(), 49);
}

#[test]
fn a_contiguous_sequence_reports_no_loss_at_all() {
    let stats = QuicLinkStats::new();
    for n in 1..=100 {
        stats.record_packet_number(n);
    }

    assert_eq!(stats.burst_loss(), 0);
    assert_eq!(stats.isolated_gaps(), 0);
}

#[test]
fn a_reordered_arrival_offsets_a_counted_gap() {
    let stats = QuicLinkStats::new();
    stats.record_packet_number(10);
    stats.record_packet_number(13);
    assert_eq!(stats.burst_loss(), 2);

    // 11 and 12 were merely late, not lost. A reordering path must not read as a lossy one.
    stats.record_packet_number(11);
    stats.record_packet_number(12);

    assert_eq!(stats.burst_loss(), 0);
    assert_eq!(stats.late_arrivals(), 2);
}

#[test]
fn the_first_packet_seen_does_not_count_everything_below_it_as_lost() {
    let stats = QuicLinkStats::new();

    // The diagnostics subscriber attaches after the handshake, so the first 1-RTT number observed is
    // routinely far above zero.
    stats.record_packet_number(50_000);

    assert_eq!(stats.burst_loss(), 0);
    assert_eq!(stats.isolated_gaps(), 0);
}

#[test]
fn a_duplicate_arrival_does_not_underflow_the_net() {
    let stats = QuicLinkStats::new();
    stats.record_packet_number(10);
    stats.record_packet_number(10);
    stats.record_packet_number(10);

    // More late arrivals than counted gaps must saturate at zero, not wrap.
    assert_eq!(stats.burst_loss(), 0);
    assert_eq!(stats.late_arrivals(), 2);
}
