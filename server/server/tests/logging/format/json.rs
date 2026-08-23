use bvc_server_lib::logging::{JsonFormatter, LogContext};
use common::curia::{Fields, Level, LogEvent};

fn line_for(fields: Fields) -> serde_json::Value {
    let context = LogContext::new_shared(None);
    let format = JsonFormatter::new(context).formatter();

    let event = LogEvent {
        level: Level::Info,
        target: "bvc_server_lib::http".to_string(),
        message: "listener bound".to_string(),
        fields,
        timestamp: chrono::Utc::now(),
        file: Some("src/http/mod.rs".to_string()),
        line: Some(103),
    };

    serde_json::from_str(&format(&event)).expect("the formatter must emit valid JSON")
}

#[test]
fn the_context_keys_are_present_and_null_without_meridian() {
    let line = line_for(Fields::new());

    assert!(line.get("instance_id").is_some_and(|v| v.is_null()));
    assert!(line.get("name").is_some_and(|v| v.is_null()));
    assert!(line["boot_id"].is_string());
}

#[test]
fn the_source_location_reaches_the_json_line() {
    let line = line_for(Fields::new());

    assert_eq!(line["file"], serde_json::json!("src/http/mod.rs"));
    assert_eq!(line["line"], serde_json::json!(103));
}

#[test]
fn call_site_fields_are_flat_not_nested() {
    let mut fields = Fields::new();
    fields.insert("bind", "127.0.0.1:8443");

    let line = line_for(fields);

    assert_eq!(line["bind"], serde_json::json!("127.0.0.1:8443"));
    assert!(line.get("fields").is_none());
}

#[test]
fn a_field_cannot_shadow_a_reserved_key() {
    let mut fields = Fields::new();
    fields.insert("msg", "impostor");
    fields.insert("boot_id", "impostor");

    let line = line_for(fields);

    assert_eq!(line["msg"], serde_json::json!("listener bound"));
    assert_ne!(line["boot_id"], serde_json::json!("impostor"));
}

#[test]
fn the_json_sink_is_never_coloured() {
    let mut fields = Fields::new();
    fields.insert("bind", "127.0.0.1:8443");

    let context = LogContext::new_shared(None);
    let format = JsonFormatter::new(context).formatter();
    let event = LogEvent {
        level: Level::Error,
        target: "t".to_string(),
        message: "m".to_string(),
        fields,
        timestamp: chrono::Utc::now(),
        file: None,
        line: None,
    };

    assert!(!format(&event).contains('\u{1b}'));
}
