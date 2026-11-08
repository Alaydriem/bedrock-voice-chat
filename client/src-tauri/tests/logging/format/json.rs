use bvc_client_lib::logging::JsonFormatter;
use tauri_plugin_curia::curia::{Fields, Level, LogEvent};

fn line(event: &LogEvent) -> serde_json::Value {
    let formatter = JsonFormatter::new().formatter();
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
    let parsed = line(&event());

    assert_eq!(parsed["device_host"], serde_json::json!("asio"));
    assert_eq!(parsed["frames"], serde_json::json!(1024));
    assert!(parsed.get("fields").is_none());
}

#[test]
fn the_envelope_is_always_present() {
    let parsed = line(&event());

    assert_eq!(parsed["level"], serde_json::json!("error"));
    assert_eq!(parsed["msg"], serde_json::json!("capture died"));
    assert_eq!(parsed["target"], serde_json::json!("bvc_client_lib::audio"));
    assert!(parsed["ts"].as_str().unwrap().contains('T'));
}

#[test]
fn a_field_cannot_shadow_an_envelope_key() {
    let mut fields = Fields::new();
    fields.insert("msg", "impostor");

    let mut e = event();
    e.fields = fields;

    let parsed = line(&e);

    assert_eq!(parsed["msg"], serde_json::json!("capture died"));
}
