use std::sync::Arc;

use curia::LogEvent;
use tauri_plugin_curia::LineFormatter;

// stderr is read by eye; the JSON file is read by jq. A field that does not fit
// on a line is named rather than printed, so third-party span fields like a
// Tauri AppHandle cannot drown the console. The full value is still in the file.
const MAX_INLINE_VALUE: usize = 96;

pub struct HumanFormatter;

impl HumanFormatter {
    pub fn new() -> Self {
        Self
    }

    pub fn formatter(self) -> LineFormatter {
        Arc::new(|event: &LogEvent| {
            let mut line = format!(
                "[{}][{}][{}] {}",
                event.timestamp.format("%H:%M:%S%.3f"),
                event.target,
                event.level.as_str().to_uppercase(),
                event.message
            );

            for (key, value) in event.fields.as_map() {
                let rendered = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };

                if rendered.len() > MAX_INLINE_VALUE {
                    line.push_str(&format!(" {key}=<{} bytes>", rendered.len()));
                } else {
                    line.push_str(&format!(" {key}={rendered}"));
                }
            }

            line
        })
    }
}

impl Default for HumanFormatter {
    fn default() -> Self {
        Self::new()
    }
}
