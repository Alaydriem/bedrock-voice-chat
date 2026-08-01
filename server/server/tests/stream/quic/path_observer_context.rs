use bvc_server_lib::stream::quic::PathObserverContext;

// s2n-quic allows five paths per connection and reclaims none of them. Warning at
// four is the last point where a log line can precede the silent drops rather than
// explain them afterwards.
#[test]
fn the_warning_threshold_sits_one_below_the_transport_limit() {
    assert_eq!(PathObserverContext::NEAR_LIMIT_THRESHOLD, 4);

    assert!(!PathObserverContext::is_near_limit(1));
    assert!(!PathObserverContext::is_near_limit(3));
    assert!(PathObserverContext::is_near_limit(4));
    assert!(PathObserverContext::is_near_limit(5));
}

// The count is what turns a series of unremarkable path-created events into the
// signal that a translator is rotating a client's source address.
#[test]
fn each_recorded_path_advances_the_count() {
    let mut context = PathObserverContext::new();

    assert_eq!(context.count(), 0);
    assert_eq!(context.record_path(), 1);
    assert_eq!(context.record_path(), 2);
    assert_eq!(context.count(), 2);
}
