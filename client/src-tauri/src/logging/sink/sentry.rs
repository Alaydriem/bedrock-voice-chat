use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tauri_plugin_curia::curia::{Level, LogEvent, Sink};

use crate::logging::{LogContext, LogThrottle, Telemetry, ThrottleDecision};

use super::{QueuedEvent, SentryWorker};

// Sized for storm headroom after throttling, not for throughput. Logging is not
// an audio pipeline.
const QUEUE_CAPACITY: usize = 1024;

const THROTTLE_WINDOW_SECS: u64 = 30;

// Third-party protocol crates. They are verbose by design and their traffic is
// diagnostic for the Connect pane, not for us. Left in, they both flood the Logs
// stream and push useful entries out of the SDK's breadcrumb buffer.
const EXCLUDED_TARGETS: &[&str] = &[
    "bedrock_protocol",
    "bedrock_client",
    "bedrock_network",
    "bedrock_server",
    "rust_raknet",
    "raknet",
    "rakrs",
];

pub struct SentrySink {
    telemetry: Arc<Telemetry>,
    throttle: LogThrottle,
    level: Level,
    tx: flume::Sender<QueuedEvent>,
    dropped: AtomicU64,
    worker: SentryWorker,
}

impl SentrySink {
    pub fn new(telemetry: Arc<Telemetry>, context: Arc<LogContext>, level: Level) -> Self {
        Self::with_capacity(telemetry, context, level, QUEUE_CAPACITY, false)
    }

    pub fn with_capacity(
        telemetry: Arc<Telemetry>,
        context: Arc<LogContext>,
        level: Level,
        capacity: usize,
        start_paused: bool,
    ) -> Self {
        let (tx, rx) = flume::bounded::<QueuedEvent>(capacity);
        let worker = SentryWorker::spawn(rx, context, start_paused);

        Self {
            telemetry,
            throttle: LogThrottle::new(),
            level,
            tx,
            dropped: AtomicU64::new(0),
            worker,
        }
    }

    pub fn window_secs() -> u64 {
        THROTTLE_WINDOW_SECS
    }

    // Error sorts lowest, so "warn or worse" is <= Warn
    fn is_log_worthy(level: Level) -> bool {
        level <= Level::Warn
    }

    pub fn is_excluded(target: &str) -> bool {
        EXCLUDED_TARGETS.iter().any(|t| target.starts_with(t))
    }

    // Events lost to a full queue. Never discarded without a count.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn queued(&self) -> usize {
        self.tx.len()
    }

    // Deliveries that panicked inside the Sentry SDK. The worker survives them.
    pub fn panics(&self) -> u64 {
        self.worker.panics()
    }

    pub fn shutdown(&self) {
        self.worker.drain_and_stop();
    }

    fn enqueue(&self, event: LogEvent, suppressed: u32, as_log: bool) {
        // try_send, never send. A blocked producer may be an audio thread.
        if self
            .tx
            .try_send(QueuedEvent {
                event,
                suppressed,
                as_log,
            })
            .is_err()
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Sink for SentrySink {
    fn level(&self) -> Level {
        self.level
    }

    fn emit(&self, event: &LogEvent) {
        if !self.telemetry.is_enabled() {
            return;
        }

        if Self::is_excluded(&event.target) {
            return;
        }

        // Everything becomes a breadcrumb. The SDK caps them at 100 per scope
        // and never transmits them on their own, so quiet traffic costs nothing
        // until an Issue is captured, and then it is exactly the trail wanted.
        if !Self::is_log_worthy(event.level) {
            self.enqueue(event.clone(), 0, false);
            return;
        }

        // A warning or worse also reaches the Logs stream, throttled so a
        // repeating failure cannot flood it.
        let suppressed = match self.throttle.evaluate(event) {
            ThrottleDecision::Suppress => return,
            ThrottleDecision::Emit { suppressed } => suppressed,
        };

        self.enqueue(event.clone(), suppressed, true);
    }
}
