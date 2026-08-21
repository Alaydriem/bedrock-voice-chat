use std::sync::Arc;

use common::structs::bedrock::BedrockLogEntry;
use curia::{Level, LogEvent, Sink};
use tokio::sync::broadcast;

use crate::bedrock::proxy::log::LogFilter;

pub struct BedrockSink {
    sender: Arc<broadcast::Sender<BedrockLogEntry>>,
    level: Level,
}

impl BedrockSink {
    pub fn new(sender: Arc<broadcast::Sender<BedrockLogEntry>>, level: Level) -> Self {
        Self { sender, level }
    }
}

impl Sink for BedrockSink {
    fn level(&self) -> Level {
        self.level
    }

    fn emit(&self, event: &LogEvent) {
        if !LogFilter::target_matches(&event.target) {
            return;
        }

        if event.message.is_empty() {
            return;
        }

        // The view renders timestamp, level and message as separate columns, so
        // the message must stay bare. The old log path passed the formatted
        // string here and the prefix was rendered twice.
        let _ = self.sender.send(BedrockLogEntry {
            timestamp_ms: event.timestamp.timestamp_millis(),
            // Uppercase: log::Level and tracing::Level both render this way, and
            // the webview manager matches on the exact string.
            level: event.level.as_str().to_uppercase(),
            target: event.target.clone(),
            message: event.message.clone(),
        });
    }
}
