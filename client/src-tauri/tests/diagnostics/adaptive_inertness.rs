use bvc_client_lib::{AdaptationEngine, MetricsCollector};
use common::structs::metrics::TransportKind;

// The capacity a jitter buffer is actually built with: `buffer_size_ms` is hardcoded to 120
// where packets are routed, and the buffer divides by the 20 ms frame duration.
const BASE_CAPACITY: usize = 6;

// These tests pin what wiring the underrun counter does and does not change within the first
// adjustment interval. Congestion assessment becomes real. Capacity and warmup do not move inside
// `AdaptiveBufferState`'s 500 ms rate limit, which is the window these tests run in. Over longer
// sessions capacity steps from 120 toward the 60 floor; it is an overflow backstop, not the
// playout depth.

#[test]
fn real_underruns_do_not_move_capacity_or_warmup() {
    let mut engine = AdaptationEngine::new(BASE_CAPACITY, TransportKind::Quic);
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
