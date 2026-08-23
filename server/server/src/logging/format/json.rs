use std::sync::Arc;

use common::curia::{LineFormatter, LogEvent};
use serde_json::{Map, Value};

use crate::logging::LogContext;

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

        let keys = self.context.snapshot();
        line.insert(
            "instance_id".to_string(),
            keys.instance_id
                .map(|id| Value::Number(id.into()))
                .unwrap_or(Value::Null),
        );
        line.insert(
            "name".to_string(),
            keys.name.map(Value::String).unwrap_or(Value::Null),
        );
        line.insert("boot_id".to_string(), Value::String(keys.boot_id));

        line.insert(
            "file".to_string(),
            event.file.clone().map(Value::String).unwrap_or(Value::Null),
        );
        line.insert(
            "line".to_string(),
            event
                .line
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );

        // Flat, so jq reads `.bind` rather than `.fields.bind`. A reserved key
        // cannot be shadowed by a field.
        for (key, value) in event.fields.as_map() {
            if !Self::is_reserved(key) {
                line.insert(key.clone(), value.clone());
            }
        }

        serde_json::to_string(&Value::Object(line)).unwrap_or_else(|e| {
            format!(r#"{{"level":"error","msg":"log serialization failed: {e}"}}"#)
        })
    }

    fn is_reserved(key: &str) -> bool {
        matches!(
            key,
            "ts" | "level" | "target" | "msg" | "instance_id" | "name" | "boot_id" | "file" | "line"
        )
    }
}
