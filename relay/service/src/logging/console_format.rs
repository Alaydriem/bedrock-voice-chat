use common::curia::{Level, LineFormatter, LogEvent};

// One line per event, for a process whose whole output is a container log.
//
// Deliberately not the BVC server's formatter: that one carries a Meridian context and
// a JSON file sink beside it. The registry writes nothing to disk, so stderr is the
// entire logging story and a line has to be readable on its own.
pub struct ConsoleFormat;

impl ConsoleFormat {
    pub fn formatter() -> LineFormatter {
        std::sync::Arc::new(Self::line)
    }

    fn line(event: &LogEvent) -> String {
        let mut line = format!(
            "{} {:>5} {}",
            event.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            Self::level(event.level),
            event.message
        );

        // Appended as `key=value` rather than as JSON. Every structured field the
        // registry logs is a peerlink, a node id or a hostname — values a person reads
        // and copies, not ones a parser consumes.
        for (key, value) in event.fields.as_map() {
            let rendered = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            line.push_str(&format!(" {key}={rendered}"));
        }

        line
    }

    pub fn level(level: Level) -> &'static str {
        match level {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }
}
