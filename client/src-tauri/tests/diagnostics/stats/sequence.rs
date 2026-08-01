use bvc_client_lib::diagnostics::QuicLinkStats;

// The envelope sequence, distinct from the QUIC packet-number path in the same struct.
//
// The server assigns this per connection at the moment it queues a datagram and nothing skips it, so
// unlike a QUIC packet number a single missing value here IS loss. These tests pin that difference.

fn loss_pct(stats: &QuicLinkStats) -> Option<f32> {
    stats.downlink_loss().map(|(lost, received)| {
        let total = lost + received;
        if total == 0 {
            0.0
        } else {
            (lost as f64 / total as f64 * 100.0) as f32
        }
    })
}

#[test]
fn loss_is_unmeasured_until_a_stamped_envelope_arrives() {
    let stats = QuicLinkStats::new();

    // None, not Some(0.0). A server predating the field must not read as a flawless link — that is
    // the failure mode this whole design exists to avoid.
    assert_eq!(stats.downlink_loss(), None);
}

#[test]
fn a_contiguous_sequence_reports_no_loss() {
    let stats = QuicLinkStats::new();
    for n in 1..=100 {
        stats.record_sequence(n);
    }

    assert_eq!(loss_pct(&stats), Some(0.0));
}

#[test]
fn a_single_gap_is_loss_because_nothing_skips_this_counter() {
    let stats = QuicLinkStats::new();
    stats.record_sequence(1);
    stats.record_sequence(3);

    // The contrast with the QUIC packet-number path: there a lone gap is ambiguous because the peer
    // skips deliberately. Here the server owns the counter and never skips, so this is loss.
    let (lost, received) = stats.downlink_loss().expect("measured");
    assert_eq!(lost, 1);
    assert_eq!(received, 2);
}

#[test]
fn the_first_sequence_seen_is_the_baseline() {
    let stats = QuicLinkStats::new();

    // Diagnostics attach after the handshake, so the first sequence observed is routinely far above
    // zero. Counting everything below it would report the whole session so far as lost.
    stats.record_sequence(50_000);

    assert_eq!(stats.downlink_loss(), Some((0, 1)));
}

#[test]
fn reordering_that_fills_a_gap_nets_to_no_loss() {
    let stats = QuicLinkStats::new();
    stats.record_sequence(1);
    stats.record_sequence(4);
    assert_eq!(stats.downlink_loss().unwrap().0, 2);

    stats.record_sequence(2);
    stats.record_sequence(3);

    // A reordering path must not read as a lossy one.
    assert_eq!(stats.downlink_loss().unwrap().0, 0);
}

#[test]
fn a_rollover_rebaselines_rather_than_reporting_four_billion_lost() {
    let stats = QuicLinkStats::new();
    stats.record_sequence(u32::MAX - 1);
    stats.record_sequence(u32::MAX);
    stats.record_sequence(0);
    stats.record_sequence(1);

    // Unreachable at ~50 datagrams/s — roughly 2.7 years on one connection — but misclassifying the
    // wrap as an enormous backward jump would silently zero the loss figure from then on.
    let (lost, _) = stats.downlink_loss().expect("measured");
    assert_eq!(lost, 0, "a wrap is not loss");
}

#[test]
fn the_rate_stays_correct_at_high_loss() {
    let stats = QuicLinkStats::new();

    // Nine in ten lost. This is the property the rejected server-report design could not have: the
    // arriving tenth still carries the gaps that prove the rest went missing.
    for n in 0..100u32 {
        stats.record_sequence(n * 10);
    }

    let pct = loss_pct(&stats).expect("measured");
    assert!(
        (pct - 90.0).abs() < 1.0,
        "expected ~90% loss to be reported as such, got {pct}"
    );
}

#[test]
fn duplicates_do_not_underflow_the_net() {
    let stats = QuicLinkStats::new();
    stats.record_sequence(5);
    stats.record_sequence(5);
    stats.record_sequence(5);

    assert_eq!(stats.downlink_loss().unwrap().0, 0);
}

#[test]
fn the_envelope_sequence_and_the_quic_packet_number_paths_are_independent() {
    let stats = QuicLinkStats::new();

    // A gap in one must not appear in the other: they measure the same thing at different layers and
    // with different confidence, and conflating them would double-count.
    stats.record_sequence(1);
    stats.record_sequence(3);
    stats.record_packet_number(100);
    stats.record_packet_number(200);

    assert_eq!(stats.downlink_loss().unwrap().0, 1);
    assert_eq!(stats.burst_loss(), 99);
}
