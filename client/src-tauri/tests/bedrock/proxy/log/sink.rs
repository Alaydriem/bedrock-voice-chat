use std::sync::Arc;

use bvc_client_lib::logging::BedrockSink;
use common::structs::bedrock::BedrockLogEntry;
use curia::{Fields, Level, LogEvent, Sink};
use tokio::sync::broadcast;

fn event(target: &str, message: &str) -> LogEvent {
    LogEvent {
        level: Level::Info,
        target: target.to_string(),
        message: message.to_string(),
        fields: Fields::new(),
        timestamp: chrono::Utc::now(),
        file: None,
        line: None,
    }
}

fn sink() -> (BedrockSink, broadcast::Receiver<BedrockLogEntry>) {
    let (tx, rx) = broadcast::channel(64);
    (BedrockSink::new(Arc::new(tx), Level::Debug), rx)
}

#[test]
fn a_matching_target_reaches_the_channel() {
    let (sink, mut rx) = sink();

    sink.emit(&event("bedrock_protocol::client", "connected"));

    let entry = rx.try_recv().expect("entry");
    assert_eq!(entry.message, "connected");
    assert_eq!(entry.target, "bedrock_protocol::client");
}

#[test]
fn an_unrelated_target_is_ignored() {
    let (sink, mut rx) = sink();

    sink.emit(&event("bvc_client_lib::audio", "capture died"));

    assert!(rx.try_recv().is_err());
}

#[test]
fn the_message_carries_no_timestamp_or_level_prefix() {
    let (sink, mut rx) = sink();

    sink.emit(&event("rust_raknet", "session opened"));

    let entry = rx.try_recv().expect("entry");
    // The view renders timestamp, level and message as separate columns, so a
    // prefix here would be rendered twice. The old log path did exactly that.
    assert_eq!(entry.message, "session opened");
}

#[test]
fn an_empty_message_is_not_forwarded() {
    let (sink, mut rx) = sink();

    sink.emit(&event("bedrock_protocol", ""));

    assert!(rx.try_recv().is_err());
}
