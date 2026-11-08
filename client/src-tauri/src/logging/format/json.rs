use std::sync::Arc;

use serde_json::{Map, Value};
use tauri_plugin_curia::LineFormatter;
use tauri_plugin_curia::curia::LogEvent;

use crate::logging::Vocabulary;

pub struct JsonFormatter;

impl JsonFormatter {
    pub fn new() -> Self {
        Self
    }

    pub fn formatter(self) -> LineFormatter {
        Arc::new(move |event: &LogEvent| self.line(event))
    }

    fn line(&self, event: &LogEvent) -> String {
        let mut line = Map::new();

        line.insert(
            "ts".to_string(),
            Value::String(
                event
                    .timestamp
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            ),
        );
        line.insert(
            "level".to_string(),
            Value::String(event.level.as_str().to_string()),
        );
        line.insert("target".to_string(), Value::String(event.target.clone()));
        line.insert("msg".to_string(), Value::String(event.message.clone()));

        // Flat, so jq reads `.device_host` rather than `.fields.device_host`.
        // Reserved keys cannot be shadowed by a field.
        for (key, value) in event.fields.as_map() {
            if !Vocabulary::is_reserved(key) {
                line.insert(key.clone(), value.clone());
            }
        }

        serde_json::to_string(&Value::Object(line)).unwrap_or_else(|e| {
            format!(r#"{{"level":"error","msg":"log serialization failed: {e}"}}"#)
        })
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}
