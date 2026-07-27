use std::sync::atomic::AtomicBool;

use log::{Log, Metadata, Record};

use super::throttle::{LogThrottle, ThrottleDecision};

pub struct SentryLogger {
    enabled: AtomicBool,
    throttle: LogThrottle,
}

impl SentryLogger {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            throttle: LogThrottle::new(),
        }
    }

    pub fn set(&self, enabled: bool) {
        self.enabled
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Log for SentryLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn log(&self, record: &Record) {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        if !sentry::Hub::current()
            .client()
            .map(|c| c.is_enabled())
            .unwrap_or(false)
        {
            return;
        }

        match record.level() {
            log::Level::Error => {
                let suppressed = match self.throttle.evaluate(record) {
                    ThrottleDecision::Suppress => return,
                    ThrottleDecision::Emit { suppressed } => suppressed,
                };

                let mut log = sentry::integrations::log::log_from_record(record);
                if suppressed > 0 {
                    log.body = format!(
                        "{} [+{} identical suppressed in last {}s]",
                        log.body,
                        suppressed,
                        self.throttle.window_secs()
                    );
                }
                sentry::Hub::current().capture_log(log);

                sentry::add_breadcrumb(sentry::integrations::log::breadcrumb_from_record(record));
            }
            _ => {
                sentry::add_breadcrumb(sentry::integrations::log::breadcrumb_from_record(record));
            }
        }
    }

    fn flush(&self) {}
}
