use std::sync::Arc;

use bvc_client_lib::logging::{JsonFormatter, LogContext};
use curia::{Fields, Level, LogEvent};

fn line(context: Arc<LogContext>, event: &LogEvent) -> serde_json::Value {
    let formatter = JsonFormatter::new(context).formatter();
    serde_json::from_str(&formatter(event)).expect("valid JSON")
}

fn event() -> LogEvent {
    let mut fields = Fields::new();
    fields.insert("device_host", "asio");
    fields.insert("frames", 1024);

    LogEvent {
        level: Level::Error,
        target: "bvc_client_lib::audio".to_string(),
        message: "capture died".to_string(),
        fields,
        timestamp: chrono::Utc::now(),
        file: None,
        line: None,
    }
}

#[test]
fn fields_are_flat_not_nested_under_a_fields_key() {
    let parsed = line(LogContext::new_shared(), &event());

    assert_eq!(parsed["device_host"], serde_json::json!("asio"));
    assert_eq!(parsed["frames"], serde_json::json!(1024));
    assert!(parsed.get("fields").is_none());
}

#[test]
fn the_envelope_is_always_present() {
    let parsed = line(LogContext::new_shared(), &event());

    assert_eq!(parsed["level"], serde_json::json!("error"));
    assert_eq!(parsed["msg"], serde_json::json!("capture died"));
    assert_eq!(parsed["target"], serde_json::json!("bvc_client_lib::audio"));
    assert!(parsed["ts"].as_str().unwrap().contains('T'));
}

#[test]
fn correlation_keys_are_null_before_setup_rather_than_absent() {
    let parsed = line(LogContext::new_shared(), &event());

    assert!(parsed.get("platform_id").is_some());
    assert!(parsed["platform_id"].is_null());
    assert!(parsed["install_id"].is_null());
    assert!(parsed["session_id"].is_null());
}

#[test]
fn correlation_keys_appear_once_set() {
    let context = LogContext::new_shared();
    context.set("p".to_string(), "i".to_string(), "s".to_string());

    let parsed = line(context, &event());

    assert_eq!(parsed["platform_id"], serde_json::json!("p"));
}

#[test]
fn a_field_cannot_shadow_an_envelope_key() {
    let mut fields = Fields::new();
    fields.insert("msg", "impostor");

    let mut e = event();
    e.fields = fields;

    let parsed = line(LogContext::new_shared(), &e);

    assert_eq!(parsed["msg"], serde_json::json!("capture died"));
}
