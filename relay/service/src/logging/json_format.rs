use common::curia::{LineFormatter, LogEvent};

use super::console_format::ConsoleFormat;
use serde_json::json;

// One JSON object per line, for the rotating file.
//
// A second format rather than the console line written to two places: the console is
// read by a person watching a start, and the file is read by whatever collects it
// afterwards. Neither is well served by the other's shape.
pub struct JsonFormat;

impl JsonFormat {
    pub fn formatter() -> LineFormatter {
        std::sync::Arc::new(Self::line)
    }

    fn line(event: &LogEvent) -> String {
        json!({
            "timestamp": event.timestamp.to_rfc3339(),
            "level": ConsoleFormat::level(event.level),
            "target": event.target,
            "message": event.message,
            "fields": event.fields.as_map(),
        })
        .to_string()
    }
}
