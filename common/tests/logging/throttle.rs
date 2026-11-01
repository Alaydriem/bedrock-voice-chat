use std::time::Duration;

use common::logging::LogThrottle;

#[test]
fn the_first_call_logs_with_nothing_suppressed() {
    let throttle = LogThrottle::new(Duration::from_secs(60));
    assert_eq!(throttle.should_log(), Some(0));
}

#[test]
fn calls_inside_the_interval_are_suppressed() {
    let throttle = LogThrottle::new(Duration::from_secs(60));
    throttle.should_log();
    assert_eq!(throttle.should_log(), None);
    assert_eq!(throttle.should_log(), None);
}

// The next line says how many it hid, so an operator knows the rate.
#[test]
fn the_next_line_after_the_interval_reports_the_suppressed_count() {
    let throttle = LogThrottle::new(Duration::from_millis(20));
    throttle.should_log();
    throttle.should_log();
    throttle.should_log();
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(throttle.should_log(), Some(2));
}
