use std::sync::Arc;

use bvc_client_lib::logging::{LogContext, SentrySink, Telemetry};
use tauri_plugin_curia::curia::{Fields, Level, LogEvent, Sink};

fn event(message: &str) -> LogEvent {
    LogEvent {
        level: Level::Error,
        target: "audio".to_string(),
        message: message.to_string(),
        fields: Fields::new(),
        timestamp: chrono::Utc::now(),
        file: None,
        line: None,
    }
}

#[test]
fn telemetry_disabled_short_circuits_before_the_throttle() {
    let sink = SentrySink::with_capacity(
        Arc::new(Telemetry::new(false)),
        LogContext::new_shared(),
        Level::Info,
        4,
        true,
    );

    for i in 0..100 {
        sink.emit(&event(&format!("message {i}")));
    }

    assert_eq!(sink.dropped(), 0);
    assert_eq!(sink.queued(), 0);
}

#[test]
fn a_full_queue_drops_and_counts_rather_than_blocking() {
    let sink = SentrySink::with_capacity(
        Arc::new(Telemetry::new(true)),
        LogContext::new_shared(),
        Level::Info,
        4,
        true,
    );

    // Distinct without digits: the throttle collapses digit runs, so numbered
    // messages would share one fingerprint and never reach the queue.
    for i in 0..64 {
        sink.emit(&event(&format!("distinct message {}", "x".repeat(i + 1))));
    }

    assert!(sink.dropped() > 0, "a full queue must drop");
    assert!(sink.queued() <= 4, "the queue must not grow past its bound");
}

#[test]
fn the_throttle_runs_before_the_queue() {
    let sink = SentrySink::with_capacity(
        Arc::new(Telemetry::new(true)),
        LogContext::new_shared(),
        Level::Info,
        4,
        true,
    );

    // One fingerprint, repeated far past the queue bound. Were the throttle
    // after the queue this would overflow; before it, exactly one is admitted.
    for _ in 0..64 {
        sink.emit(&event("identical message"));
    }

    assert_eq!(sink.dropped(), 0);
    assert_eq!(sink.queued(), 1);
}

#[test]
fn quiet_traffic_is_queued_as_a_breadcrumb_rather_than_discarded() {
    let sink = SentrySink::with_capacity(
        Arc::new(Telemetry::new(true)),
        LogContext::new_shared(),
        Level::Debug,
        64,
        true,
    );

    let mut info = event("player joined");
    info.level = Level::Info;
    sink.emit(&info);

    // Breadcrumbs are what give an Issue its trail, so quiet traffic still has
    // to reach the worker. It simply does not become a Sentry Log.
    assert_eq!(sink.queued(), 1);
    assert_eq!(sink.dropped(), 0);
}

#[test]
fn quiet_traffic_is_not_throttled() {
    let sink = SentrySink::with_capacity(
        Arc::new(Telemetry::new(true)),
        LogContext::new_shared(),
        Level::Debug,
        64,
        true,
    );

    // The same info line repeated would be collapsed by the throttle if it were
    // on the log path. As breadcrumbs, every one is kept.
    for _ in 0..10 {
        let mut info = event("identical info");
        info.level = Level::Info;
        sink.emit(&info);
    }

    assert_eq!(sink.queued(), 10);
}

#[test]
fn a_repeated_warning_is_throttled_on_the_log_path() {
    let sink = SentrySink::with_capacity(
        Arc::new(Telemetry::new(true)),
        LogContext::new_shared(),
        Level::Debug,
        64,
        true,
    );

    for _ in 0..10 {
        let mut warning = event("identical warning");
        warning.level = Level::Warn;
        sink.emit(&warning);
    }

    assert_eq!(sink.queued(), 1);
}
