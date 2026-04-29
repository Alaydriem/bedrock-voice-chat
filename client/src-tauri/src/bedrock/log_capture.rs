use std::sync::{Arc, OnceLock};

use common::structs::bedrock::BedrockLogEntry;
use tokio::sync::broadcast;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

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

static SENDER: OnceLock<Arc<broadcast::Sender<BedrockLogEntry>>> = OnceLock::new();

pub struct BedrockLogChannel {
    sender: Arc<broadcast::Sender<BedrockLogEntry>>,
}

impl BedrockLogChannel {
    pub fn init() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        let arc = Arc::new(tx);
        let _ = SENDER.set(arc.clone());
        Self { sender: arc }
    }

    pub fn sender(&self) -> Arc<broadcast::Sender<BedrockLogEntry>> {
        Arc::clone(&self.sender)
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

fn emit_entry(entry: BedrockLogEntry) {
    if let Some(tx) = SENDER.get() {
        let _ = tx.send(entry);
    }
}

pub struct BedrockLogger;

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
        emit_entry(BedrockLogEntry {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            level: record.level().to_string(),
            target: record.target().to_string(),
            message: record.args().to_string(),
        });
    }

    fn flush(&self) {}
}

pub struct BedrockTracingLayer;

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
        emit_entry(BedrockLogEntry {
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
