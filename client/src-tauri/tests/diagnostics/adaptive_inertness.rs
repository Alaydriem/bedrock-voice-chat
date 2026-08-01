use bvc_client_lib::{AdaptationEngine, MetricsCollector};

// The capacity a jitter buffer is actually built with: `buffer_size_ms` is hardcoded to 120
// where packets are routed, and the buffer divides by the 20 ms frame duration.
const BASE_CAPACITY: usize = 6;

// `record_underrun` had no caller before this work, so `NetworkMetrics::buffer_underruns` was
// permanently zero and `CongestionLevel::from_buffer_metrics` read a constant. It now has one.
//
// These tests pin what that does and does not change. It makes congestion assessment real. It
// does NOT move the buffer, because `AdaptiveBufferState` clamps every target to a 60 frame
// floor while the base capacity is 6 — so every reachable multiplier produces the same clamped
// target and `adjust_capacity` always declines.
//
// This inertness is recorded rather than fixed: the unit confusion behind it belongs to the
// buffer redesign. Pinning it here means the redesign inherits a stated fact instead of
// rediscovering it, and learns immediately if it changes.

#[test]
fn real_underruns_do_not_move_capacity_or_warmup() {
    let mut engine = AdaptationEngine::new(BASE_CAPACITY);
    let mut metrics = MetricsCollector::default();

    let capacity_before = engine.current_capacity();
    let warmup_before = engine.warmup_packets_needed();

    // A severe, sustained underrun run — far past every CongestionLevel threshold.
    for i in 0..500u64 {
        metrics.record_underrun();
        metrics.record_packet_arrival(i * 20, 0);
        metrics.record_silence_generation();
    }

    engine.assess_network_conditions(&metrics);
    let adjustment = engine.adjust_buffer_if_needed(&metrics);

    assert_eq!(
        adjustment, None,
        "the min_capacity clamp must swallow every reachable multiplier"
    );
    assert_eq!(
        engine.current_capacity(),
        capacity_before,
        "capacity must not move"
    );
    assert_eq!(
        engine.warmup_packets_needed(),
        warmup_before,
        "warmup requirement must not move"
    );
}

#[test]
fn underruns_are_recorded_where_the_congestion_assessment_can_read_them() {
    let mut metrics = MetricsCollector::default();
    assert_eq!(metrics.network_metrics.buffer_underruns, 0);

    metrics.record_underrun();
    metrics.record_underrun();

    // The point of writing to MetricsCollector at all: this field feeds
    // CongestionLevel::from_buffer_metrics, which previously only ever saw zero.
    assert_eq!(metrics.network_metrics.buffer_underruns, 2);
}

#[test]
fn reorder_tolerance_is_unaffected_by_underruns() {
    // A controlled comparison, not a before-and-after: `assess_network_conditions` moves the
    // quality off its `Good` construction default regardless of underruns, so comparing
    // pre-assessment to post-assessment would measure that transition instead of the variable
    // under test.
    let mut with_underruns = AdaptationEngine::new(BASE_CAPACITY);
    let mut without_underruns = AdaptationEngine::new(BASE_CAPACITY);
    let mut metrics_with = MetricsCollector::default();
    let mut metrics_without = MetricsCollector::default();

    for i in 0..500u64 {
        metrics_with.record_underrun();
        metrics_with.record_packet_arrival(i * 20, 0);
        metrics_without.record_packet_arrival(i * 20, 0);
    }

    with_underruns.assess_network_conditions(&metrics_with);
    without_underruns.assess_network_conditions(&metrics_without);

    // Reorder tolerance is the one adaptive output read ungated during packet acceptance, and
    // it derives from NetworkQuality — which underruns do not feed. Packet acceptance is
    // therefore untouched by wiring the underrun counter.
    assert_eq!(
        with_underruns.reorder_window_ms(),
        without_underruns.reorder_window_ms(),
        "underruns must not change reorder tolerance, which gates packet acceptance"
    );
}
