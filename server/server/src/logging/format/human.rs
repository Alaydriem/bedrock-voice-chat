use std::io::IsTerminal;
use std::sync::Arc;

use common::curia::{Level, LineFormatter, LogEvent};

// stderr is read by eye; the JSON file is read by jq. A value that does not fit
// on a line is named rather than printed, so a large third-party span field
// cannot drown the console. The full value is still in the file.
const MAX_INLINE_VALUE: usize = 96;

// Exempt from the length limit. A peer link is the one string the far side's `peer`
// block requires and the startup log is where an operator reads it; naming its length
// instead of printing it leaves them with nothing to copy.
const VERBATIM_KEYS: &[&str] = &["peerlink"];

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

pub struct HumanFormatter {
    color: bool,
}

impl HumanFormatter {
    pub fn new(color: bool) -> Self {
        Self { color }
    }

    // Decided once at construction, not per line. anstyle-query implements the
    // NO_COLOR, CLICOLOR and CLICOLOR_FORCE conventions and the TERM=dumb case.
    pub fn detect_color() -> bool {
        anstyle_query::clicolor_force()
            || (std::io::stderr().is_terminal()
                && !anstyle_query::no_color()
                && anstyle_query::term_supports_color())
    }

    pub fn formatter(self) -> LineFormatter {
        Arc::new(move |event: &LogEvent| self.line(event))
    }

    fn level_sgr(level: Level) -> &'static str {
        match level {
            Level::Error => "\x1b[1;31m",
            Level::Warn => "\x1b[1;33m",
            Level::Info => "\x1b[1;32m",
            Level::Debug => "\x1b[1;34m",
            Level::Trace => DIM,
        }
    }

    fn line(&self, event: &LogEvent) -> String {
        let time = event.timestamp.format("%H:%M:%S%.3f");
        let level = event.level.as_str().to_uppercase();

        let mut line = if self.color {
            format!(
                "{DIM}[{time}][{}]{RESET}[{}{level}{RESET}] {}",
                event.target,
                Self::level_sgr(event.level),
                event.message
            )
        } else {
            format!("[{time}][{}][{level}] {}", event.target, event.message)
        };

        for (key, value) in event.fields.as_map() {
            let rendered = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };

            let rendered = if rendered.len() > MAX_INLINE_VALUE
                && !VERBATIM_KEYS.contains(&key.as_str())
            {
                format!("<{} bytes>", rendered.len())
            } else {
                rendered
            };

            if self.color {
                line.push_str(&format!(" {DIM}{key}={RESET}{rendered}"));
            } else {
                line.push_str(&format!(" {key}={rendered}"));
            }
        }

        line
    }
}
