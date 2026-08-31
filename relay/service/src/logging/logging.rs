use common::curia::{
    ConsoleSink, Dispatcher, FileOpenStrategy, FileSink, Filter, Level, Logger, RotationStrategy,
    TimezoneStrategy, TracingBridge,
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::LoggerConfig;

use super::console_format::ConsoleFormat;
use super::json_format::JsonFormat;
use super::sink::LogSinkType;

// Installs the logging pipeline: a human console on stderr and a rotating JSON file,
// the same arrangement the BVC server uses.
//
// Without this every `curia::info!` in the registry is discarded and the process runs
// silently — the enrollment peerlink an operator has nowhere else to read, the bound
// address, and the minutes spent waiting on DNS propagation all vanish. A registry that
// looks hung and one that is working are then indistinguishable.
pub struct Logging;

impl Logging {
    const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
    const ARCHIVES_KEPT: usize = 10;
    const FILE_STEM: &'static str = "bvc-relay";

    // Installed exactly once, after the config is read.
    //
    // Not installed earlier as console-only and upgraded later: `Logger::install` is a
    // `OnceLock`, so the first call wins and the second is rejected. A console-first
    // arrangement would therefore silently discard the file sink — which reads as
    // "logging is configured but no file ever appears".
    //
    // A config that fails to parse never reaches this, and is reported by `main`
    // returning the error instead.
    pub fn install(config: LoggerConfig) {
        // Windows consoles need ENABLE_VIRTUAL_TERMINAL_PROCESSING before an escape
        // sequence renders. Cross-platform; returns None elsewhere.
        let _ = anstyle_query::windows::enable_ansi_colors();

        let directives = std::env::var("RUST_LOG").unwrap_or_else(|_| config.directives());
        let filter = Filter::from_directives(&directives);

        // Trace at the sink, so the dispatcher's filter is the only authority.
        let mut sinks = vec![LogSinkType::Console(ConsoleSink::new(
            Level::Trace,
            ConsoleFormat::formatter(),
        ))];

        sinks.extend(Self::file_sink(&config.path));

        let dispatcher = Dispatcher::new(sinks).with_filter(filter.clone());

        // A second runtime in this process finds the OnceLock claimed. curia drops the
        // rejected dispatcher, which closes its file and drains its worker, so nothing
        // is retained on that path.
        if Logger::install(Box::new(dispatcher)).is_ok() {
            tracing_subscriber::registry()
                .with(TracingBridge::to_global().with_filter(filter))
                .init();
        }
    }

    // Logging must never take the registry down with it. An unwritable directory
    // degrades to console-only and says so once, on the console that still works.
    fn file_sink(path: &str) -> Option<LogSinkType> {
        let dir = std::path::PathBuf::from(path);

        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("log directory {path} unavailable, continuing without a file: {e}");
            return None;
        }

        match FileSink::with_rotation(
            dir,
            Self::FILE_STEM.to_string(),
            Level::Trace,
            JsonFormat::formatter(),
            Self::MAX_FILE_SIZE,
            RotationStrategy::KeepSome(Self::ARCHIVES_KEPT),
            TimezoneStrategy::UseUtc,
            FileOpenStrategy::Append,
        ) {
            Ok(file) => Some(LogSinkType::File(file)),
            Err(e) => {
                eprintln!("log file in {path} unavailable, continuing without it: {e}");
                None
            }
        }
    }
}
