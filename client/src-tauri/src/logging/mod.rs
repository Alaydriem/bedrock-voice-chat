mod context;
mod defect;
mod fields;
mod format;
mod sink;
mod smoke;
mod throttle;
pub mod telemetry;

pub use self::context::{CorrelationKeys, LogContext};
pub use self::defect::Defect;
pub use self::fields::{Destination, FieldSpec, RoutedFields, Vocabulary};
pub use self::format::{HumanFormatter, JsonFormatter};
#[cfg(feature = "bedrock-protocol")]
pub use self::sink::BedrockSink;
pub use self::sink::{LogSinkType, QueuedEvent, SentrySink, SentryWorker};
pub use self::smoke::LoggingSmokeTest;
pub use self::telemetry::Telemetry;
pub use self::throttle::{LogThrottle, ThrottleDecision};

// curia's own default is 40_000 bytes, which throws a debug-level session away several
// times a minute. Twenty-five megabytes holds a long call plus the launch that preceded
// it, which is the span a fault report has to cover.
pub const LOG_MAX_FILE_SIZE: u64 = 25 * 1024 * 1024;

// curia's own default is `KeepOne`, which creates no archive at all. Three archives beside
// the active file bounds the directory at 100 MB while keeping the run before last, which
// is where an intermittent fault usually is.
pub const LOG_ARCHIVES_KEPT: usize = 3;
