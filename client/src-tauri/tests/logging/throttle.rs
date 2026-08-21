use bvc_client_lib::logging::{LogThrottle, ThrottleDecision};
use tauri_plugin_curia::curia::{Fields, Level, LogEvent};

fn event(message: &str, fields: Fields) -> LogEvent {
    LogEvent {
        level: Level::Error,
        target: "audio".to_string(),
        message: message.to_string(),
        fields,
        timestamp: chrono::Utc::now(),
        file: None,
        line: None,
    }
}

fn with(key: &str, value: &str) -> Fields {
    let mut fields = Fields::new();
    fields.insert(key, value);
    fields
}

#[test]
fn an_identical_event_is_suppressed_within_the_window() {
    let throttle = LogThrottle::new();

    assert!(matches!(
        throttle.evaluate(&event("capture died", Fields::new())),
        ThrottleDecision::Emit { suppressed: 0 }
    ));
    assert!(matches!(
        throttle.evaluate(&event("capture died", Fields::new())),
        ThrottleDecision::Suppress
    ));
}

#[test]
fn digits_are_normalized_so_numeric_detail_shares_a_fingerprint() {
    let throttle = LogThrottle::new();

    throttle.evaluate(&event("failed after 3 retries", Fields::new()));

    assert!(matches!(
        throttle.evaluate(&event("failed after 47 retries", Fields::new())),
        ThrottleDecision::Suppress
    ));
}

#[test]
fn tag_fields_throttle_independently() {
    let throttle = LogThrottle::new();

    throttle.evaluate(&event("capture died", with("device_host", "asio")));

    assert!(matches!(
        throttle.evaluate(&event("capture died", with("device_host", "wasapi"))),
        ThrottleDecision::Emit { .. }
    ));
}

#[test]
fn attribute_fields_do_not_split_the_fingerprint() {
    let throttle = LogThrottle::new();

    throttle.evaluate(&event("capture died", with("device_name", "Focusrite")));

    assert!(matches!(
        throttle.evaluate(&event("capture died", with("device_name", "Behringer"))),
        ThrottleDecision::Suppress
    ));
}
