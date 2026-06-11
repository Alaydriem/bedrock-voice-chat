use std::sync::Arc;

use common::structs::bedrock::BedrockLogEntry;
use tokio::sync::broadcast;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

use super::filter::LogFilter;
use super::message_visitor::MessageVisitor;

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
        if !LogFilter::tracing_level_allowed(level) {
            return;
        }
        let target = event.metadata().target();
        if !LogFilter::target_matches(target) {
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
