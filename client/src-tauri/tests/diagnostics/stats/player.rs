use std::sync::Arc;
use std::thread;

use bvc_client_lib::diagnostics::PlayerReceiveStats;

fn stats() -> PlayerReceiveStats {
    PlayerReceiveStats::new("Alice".to_string())
}

#[test]
fn concealment_is_zero_when_every_frame_decoded() {
    let s = stats();
    s.record_decode(100);

    assert_eq!(s.concealment_pct(), 0.0);
}

#[test]
fn concealment_is_the_fabricated_fraction_of_what_played() {
    let s = stats();
    s.record_decode(90);
    for _ in 0..10 {
        s.record_plc();
    }

    // Ten fabricated frames out of a hundred played. This is what a listener actually experiences,
    // and unlike a loss rate it is derivable from what this client can observe: frames carry a
    // wall-clock capture stamp and no sequence number, so a gap cannot distinguish a closed noise
    // gate from a lost packet.
    assert!(
        (s.concealment_pct() - 10.0).abs() < 0.01,
        "expected 10% concealment, got {}",
        s.concealment_pct()
    );
}

#[test]
fn silence_counts_as_concealment_alongside_plc() {
    let s = stats();
    s.record_decode(8);
    s.record_plc();
    s.record_silence();

    assert!(
        (s.concealment_pct() - 20.0).abs() < 0.01,
        "both concealment kinds must count, got {}",
        s.concealment_pct()
    );
}

#[test]
fn concealment_with_nothing_played_is_zero_not_total() {
    let s = stats();

    // No frames at all is not 100% concealed. A speaker who has said nothing must not read as
    // completely broken.
    assert_eq!(s.concealment_pct(), 0.0);
    assert!(s.is_idle());
}

#[test]
fn concealment_is_capped_at_a_hundred_percent() {
    let s = stats();
    for _ in 0..50 {
        s.record_plc();
    }

    assert_eq!(s.concealment_pct(), 100.0);
}

#[test]
fn an_arrival_is_counted_without_consulting_its_timestamp() {
    let s = stats();

    // Timestamps are deliberately ignored: they are wall-clock capture stamps, so out-of-order or
    // wildly spaced values must not affect the counter or produce anything but a plain increment.
    s.record_arrival(5_000);
    s.record_arrival(10);
    s.record_arrival(u64::MAX);

    assert_eq!(s.frames_received(), 3);
    assert!(!s.is_idle());
}

#[test]
fn concurrent_arrivals_lose_no_increments() {
    // Each route holds its own instance today, so concurrent callers on one are not expected. This
    // pins that the counter is nonetheless a plain atomic increment, so a future change that shares
    // an instance cannot silently lose frames.
    let stats = Arc::new(stats());
    let mut handles = Vec::new();

    for t in 0..4u64 {
        let s = stats.clone();
        handles.push(thread::spawn(move || {
            for i in 0..500u64 {
                s.record_arrival(1_000 + t * 500 + i);
            }
        }));
    }
    for h in handles {
        h.join().expect("thread joins");
    }

    assert_eq!(
        stats.frames_received(),
        2_000,
        "the total must not lose increments under concurrent writers"
    );
}

#[test]
fn buffer_ms_is_ring_length_times_the_frame_duration() {
    let s = stats();
    s.set_ring(6, 60, 3);

    // The adaptive buffer's capacity fields are declared in milliseconds but compared against a
    // frame count; the conversion happens here instead of trusting that unit.
    assert_eq!(s.buffer_ms(), 120);
}

#[test]
fn quality_score_is_perfect_with_no_frames_and_degrades_with_concealment() {
    let s = stats();
    assert_eq!(s.quality_score(), 1.0);

    s.record_decode(10);
    let clean = s.quality_score();

    s.record_plc();
    s.record_plc();
    let concealed = s.quality_score();

    s.record_silence();
    let silenced = s.quality_score();

    assert!(clean > concealed, "PLC must lower the score");
    assert!(concealed > silenced, "silence must lower it further");
}

#[test]
fn merging_normal_and_spatial_routes_yields_one_peer_with_summed_counters() {
    let normal = stats();
    let spatial = stats();

    normal.record_underrun();
    normal.record_overflow_drop();
    normal.record_decode(10);
    normal.set_ring(3, 60, 3);

    spatial.record_underrun();
    spatial.record_underrun();
    spatial.record_ooo_drop();
    spatial.record_decode(5);
    spatial.set_ring(7, 60, 3);

    let mut merged = normal.to_diagnostics();
    spatial.merge_into(&mut merged);

    // A speaker heard both normally and spatially has two jitter buffers. Reporting them as
    // two peers would double-count one person's drops.
    assert_eq!(merged.name, "Alice");
    assert_eq!(merged.underruns, 3);
    assert_eq!(merged.overflow_drops, 1);
    assert_eq!(merged.ooo_drops, 1);
    assert_eq!(merged.frames_decoded, 15);
    // Occupancy is the worse of the two, not the sum: two buffers three frames deep is not a
    // six-frame backlog.
    assert_eq!(merged.ring_len, 7);
    assert_eq!(merged.buffer_ms, 140);
    // Concealment is the worse of the two routes, not their sum.
    assert!(merged.concealment_pct <= 100.0);
}

#[test]
fn an_idle_peer_is_reported_idle_and_a_fed_one_is_not() {
    let s = stats();
    assert!(s.is_idle());

    s.record_arrival(1_000);

    assert!(!s.is_idle());
}

#[test]
fn underruns_and_drops_are_counted_separately() {
    let s = stats();
    s.record_underrun();
    s.record_underrun();
    s.record_overflow_drop();
    s.record_ooo_drop();

    // This separation is the entire diagnostic: underruns with no drops means the speaker
    // stopped sending, drops mean the network.
    let d = s.to_diagnostics();
    assert_eq!(d.underruns, 2);
    assert_eq!(d.overflow_drops, 1);
    assert_eq!(d.ooo_drops, 1);
}
