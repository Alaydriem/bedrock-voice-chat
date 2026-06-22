use std::sync::Arc;

use common::structs::bedrock::BedrockLogEntry;
use tokio::sync::broadcast;

use super::filter::LogFilter;

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
        LogFilter::log_level_allowed(metadata.level())
            && LogFilter::target_matches(metadata.target())
    }

    fn log(&self, record: &log::Record) {
        if !LogFilter::log_level_allowed(record.level()) {
            return;
        }
        if !LogFilter::target_matches(record.target()) {
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
