use std::sync::atomic::AtomicBool;

use log::{Log, Metadata, Record};
use once_cell::sync::Lazy;

const BREADCRUMB_BUFFER_SIZE: usize = 100;

static BREADCRUMB_BUFFER: Lazy<(
    flume::Sender<sentry::Breadcrumb>,
    flume::Receiver<sentry::Breadcrumb>,
)> = Lazy::new(|| flume::bounded(BREADCRUMB_BUFFER_SIZE));

pub struct SentryLogger {
    enabled: AtomicBool,
}

impl SentryLogger {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
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

        if record.level() <= log::Level::Warn {
            sentry::Hub::current().capture_log(sentry::integrations::log::log_from_record(record));
        }

        match record.level() {
            log::Level::Error => {
                let (_, rx) = &*BREADCRUMB_BUFFER;
                while let Ok(breadcrumb) = rx.try_recv() {
                    sentry::add_breadcrumb(breadcrumb);
                }
                sentry::capture_event(sentry::integrations::log::exception_from_record(record));
            }
            _ => {
                let breadcrumb = sentry::integrations::log::breadcrumb_from_record(record);
                let (tx, rx) = &*BREADCRUMB_BUFFER;
                if let Err(flume::TrySendError::Full(bc)) = tx.try_send(breadcrumb) {
                    let _ = rx.try_recv();
                    let _ = tx.try_send(bc);
                }
            }
        }
    }

    fn flush(&self) {}
}
