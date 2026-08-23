use std::sync::Arc;

use tauri_plugin_curia::curia::LogEvent;
use serde_json::{Map, Value};
use tauri_plugin_curia::LineFormatter;

use crate::logging::{LogContext, Vocabulary};

pub struct JsonFormatter {
    context: Arc<LogContext>,
}

impl JsonFormatter {
    pub fn new(context: Arc<LogContext>) -> Self {
        Self { context }
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

        // Always present, null before setup resolves them. A stable schema with
        // a visible gap beats a key that is sometimes absent.
        let keys = self.context.snapshot();
        line.insert("platform_id".to_string(), Self::or_null(keys.platform_id));
        line.insert("install_id".to_string(), Self::or_null(keys.install_id));
        line.insert("session_id".to_string(), Self::or_null(keys.session_id));

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

    fn or_null(value: Option<String>) -> Value {
        value.map(Value::String).unwrap_or(Value::Null)
    }
}
