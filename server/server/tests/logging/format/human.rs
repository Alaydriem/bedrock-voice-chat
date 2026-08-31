use bvc_server_lib::logging::HumanFormatter;
use common::curia::{Fields, Level, LogEvent};

fn event(message: &str) -> LogEvent {
    LogEvent {
        level: Level::Warn,
        target: "bvc_server_lib::http".to_string(),
        message: message.to_string(),
        fields: Fields::new(),
        timestamp: chrono::Utc::now(),
        file: Some("src/http/mod.rs".to_string()),
        line: Some(103),
    }
}

#[test]
fn colour_off_emits_no_escape_sequence() {
    let format = HumanFormatter::new(false).formatter();

    let line = format(&event("listener bound"));

    assert!(!line.contains('\u{1b}'), "unexpected escape in: {line:?}");
    assert!(line.contains("[WARN] listener bound"));
    assert!(line.contains("[bvc_server_lib::http]"));
}

#[test]
fn colour_on_wraps_the_level_in_its_own_sgr() {
    let format = HumanFormatter::new(true).formatter();

    let line = format(&event("listener bound"));

    assert!(line.contains("\u{1b}[1;33mWARN\u{1b}[0m"), "got: {line:?}");
}

#[test]
fn an_oversized_field_value_is_named_rather_than_printed() {
    let mut fields = Fields::new();
    fields.insert("blob", "x".repeat(200));
    let mut e = event("big field");
    e.fields = fields;

    let line = HumanFormatter::new(false).formatter()(&e);

    assert!(line.contains("blob=<200 bytes>"), "got: {line:?}");
    assert!(!line.contains(&"x".repeat(200)));
}

#[test]
fn an_oversized_peerlink_is_printed_in_full() {
    let peerlink = format!("bvcpeer{}", "a".repeat(200));
    let mut fields = Fields::new();
    fields.insert("peerlink", peerlink.clone());
    let mut e = event("this server's peer link");
    e.fields = fields;

    let line = HumanFormatter::new(false).formatter()(&e);

    assert!(line.contains(&format!("peerlink={peerlink}")), "got: {line:?}");
}
