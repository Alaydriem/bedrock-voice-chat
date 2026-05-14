use std::sync::Arc;

use common::structs::bedrock::BedrockLogEntry;
use tokio::sync::broadcast;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

const CHANNEL_CAPACITY: usize = 512;

const TARGET_PREFIXES: &[&str] = &[
    "bedrock_protocol",
    "bedrock_server",
    "bedrock_network",
    "rust_raknet",
    "raknet",
    "rakrs",
    "bvc_client_lib::bedrock",
    "bedrock_voice_chat_client::bedrock",
];

pub struct BedrockLogChannel {
    sender: Arc<broadcast::Sender<BedrockLogEntry>>,
}

impl BedrockLogChannel {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender: Arc::new(tx),
        }
    }

    pub fn sender(&self) -> Arc<broadcast::Sender<BedrockLogEntry>> {
        Arc::clone(&self.sender)
    }
}

impl Default for BedrockLogChannel {
    fn default() -> Self {
        Self::new()
    }
}

fn target_matches(target: &str) -> bool {
    TARGET_PREFIXES.iter().any(|p| target.starts_with(p))
}

fn is_log_level_allowed(level: log::Level) -> bool {
    matches!(level, log::Level::Info | log::Level::Warn | log::Level::Error)
}

fn is_tracing_level_allowed(level: &tracing::Level) -> bool {
    *level == tracing::Level::INFO
        || *level == tracing::Level::WARN
        || *level == tracing::Level::ERROR
}

pub struct BedrockLogger {
    sender: Arc<broadcast::Sender<BedrockLogEntry>>,
}

impl BedrockLogger {
    pub fn new(sender: Arc<broadcast::Sender<BedrockLogEntry>>) -> Self {
        Self { sender }
    }

    fn emit(&self, entry: BedrockLogEntry) {
        let _ = self.sender.send(entry);
    }
}

impl log::Log for BedrockLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        is_log_level_allowed(metadata.level()) && target_matches(metadata.target())
    }

    fn log(&self, record: &log::Record) {
        if !is_log_level_allowed(record.level()) {
            return;
        }
        if !target_matches(record.target()) {
            return;
        }
        self.emit(BedrockLogEntry {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            level: record.level().to_string(),
            target: record.target().to_string(),
            message: record.args().to_string(),
        });
    }

    fn flush(&self) {}
}

pub struct BedrockTracingLayer {
    sender: Arc<broadcast::Sender<BedrockLogEntry>>,
}

impl BedrockTracingLayer {
    pub fn new(sender: Arc<broadcast::Sender<BedrockLogEntry>>) -> Self {
        Self { sender }
    }

    fn emit(&self, entry: BedrockLogEntry) {
        let _ = self.sender.send(entry);
    }
}

impl<S: Subscriber> Layer<S> for BedrockTracingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = event.metadata().level();
        if !is_tracing_level_allowed(level) {
            return;
        }
        let target = event.metadata().target();
        if !target_matches(target) {
            return;
        }
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        if visitor.message.is_empty() {
            return;
        }
        self.emit(BedrockLogEntry {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            level: level.to_string(),
            target: target.to_string(),
            message: visitor.message,
        });
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let formatted = format!("{:?}", value);
            self.message = if formatted.starts_with('"') && formatted.ends_with('"') && formatted.len() >= 2 {
                formatted[1..formatted.len() - 1].to_string()
            } else {
                formatted
            };
        }
    }
}
