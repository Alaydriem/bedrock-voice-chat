use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::logging::{Defect, LogContext, Vocabulary};

use super::QueuedEvent;

pub struct SentryWorker {
    paused: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    panics: Arc<AtomicU64>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl SentryWorker {
    // start_paused removes a race in tests: pausing after spawn lets the worker
    // drain the queue before it observes the flag.
    pub fn spawn(
        rx: flume::Receiver<QueuedEvent>,
        context: Arc<LogContext>,
        start_paused: bool,
    ) -> Self {
        let paused = Arc::new(AtomicBool::new(start_paused));
        let stop = Arc::new(AtomicBool::new(false));
        let panics = Arc::new(AtomicU64::new(0));

        let worker_paused = paused.clone();
        let worker_stop = stop.clone();
        let worker_panics = panics.clone();

        let handle = std::thread::Builder::new()
            .name("sentry-sink".to_string())
            .spawn(move || {
                while !worker_stop.load(Ordering::SeqCst) {
                    if worker_paused.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }

                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(queued) => Self::deliver_guarded(&queued, &context, &worker_panics),
                        Err(flume::RecvTimeoutError::Timeout) => continue,
                        Err(flume::RecvTimeoutError::Disconnected) => break,
                    }
                }

                // Drain before exiting, or the last events of a session are lost
                // and those are the interesting ones.
                while let Ok(queued) = rx.try_recv() {
                    Self::deliver_guarded(&queued, &context, &worker_panics);
                }
            })
            .ok();

        Self {
            paused,
            stop,
            panics,
            handle: Mutex::new(handle),
        }
    }

    // The worker must outlive any single bad event. A panic inside the Sentry
    // SDK kills only this delivery; the loop continues and the count is kept so
    // the failure is visible rather than silent.
    fn deliver_guarded(queued: &QueuedEvent, context: &LogContext, panics: &AtomicU64) {
        let result = catch_unwind(AssertUnwindSafe(|| Self::deliver(queued, context)));

        if result.is_err() {
            panics.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn panics(&self) -> u64 {
        self.panics.load(Ordering::Relaxed)
    }

    // Stops the loop and waits for the drain. Without the join the process can
    // exit before the queued tail reaches Sentry, and the tail is the part worth
    // having.
    pub fn drain_and_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.paused.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.lock().ok().and_then(|mut h| h.take()) {
            let _ = handle.join();
        }
    }

    fn deliver(queued: &QueuedEvent, context: &LogContext) {
        let routed = Vocabulary::route(&queued.event.fields);
        let keys = context.snapshot();

        let mut body = queued.event.message.clone();
        if queued.suppressed > 0 {
            body = format!(
                "{} [+{} identical suppressed in last {}s]",
                body,
                queued.suppressed,
                super::SentrySink::window_secs()
            );
        }

        #[allow(unused_mut)]
        let mut attributes: std::collections::BTreeMap<String, sentry::protocol::LogAttribute> =
            Default::default();

        for (key, value) in &routed.attributes {
            attributes.insert(key.clone(), Self::attribute(value));
        }
        for (key, value) in &routed.tags {
            attributes.insert(key.clone(), value.clone().into());
        }

        attributes.insert("logger.target".into(), queued.event.target.clone().into());
        if let Some(platform_id) = keys.platform_id.clone() {
            attributes.insert("platform_id".into(), platform_id.into());
        }
        if let Some(install_id) = keys.install_id.clone() {
            attributes.insert("install_id".into(), install_id.into());
        }
        if let Some(session_id) = keys.session_id.clone() {
            attributes.insert("session_id".into(), session_id.into());
        }

        // Ordering matters: breadcrumbs are attached to whatever Issue is
        // captured after them, so both run on this one thread rather than
        // racing between the producer and here.
        if queued.as_log {
            sentry::Hub::current().capture_log(sentry::protocol::Log {
                level: Self::log_level(queued.event.level),
                body: body.clone(),
                trace_id: None,
                timestamp: std::time::SystemTime::now(),
                severity_number: None,
                attributes,
            });
        }

        let mut data: std::collections::BTreeMap<String, serde_json::Value> = Default::default();
        for (key, value) in &routed.attributes {
            data.insert(key.clone(), value.clone());
        }
        for (key, value) in &routed.tags {
            data.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        sentry::add_breadcrumb(sentry::Breadcrumb {
            ty: "log".into(),
            level: Self::breadcrumb_level(queued.event.level),
            category: Some(queued.event.target.clone()),
            message: Some(body.clone()),
            data,
            ..Default::default()
        });

        // Only a declared defect becomes an Issue, and its fingerprint is the
        // variant rather than the message text.
        if let Some(defect) = Defect::from_fields(&queued.event.fields) {
            let context_map: std::collections::BTreeMap<String, serde_json::Value> = routed
                .context
                .iter()
                .chain(routed.attributes.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            sentry::with_scope(
                |scope| {
                    scope.set_fingerprint(Some(&[defect.as_str()]));
                    for (key, value) in &routed.tags {
                        scope.set_tag(key, value);
                    }
                    scope.set_context(
                        "fields",
                        sentry::protocol::Context::Other(context_map.clone()),
                    );
                },
                || {
                    sentry::capture_event(sentry::protocol::Event {
                        level: Self::breadcrumb_level(queued.event.level),
                        logger: Some(queued.event.target.clone()),
                        message: Some(body.clone()),
                        ..Default::default()
                    });
                },
            );
        }
    }

    fn attribute(value: &serde_json::Value) -> sentry::protocol::LogAttribute {
        match value {
            serde_json::Value::String(s) => s.clone().into(),
            serde_json::Value::Bool(b) => (*b).into(),
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(i) => i.into(),
                None => n.as_f64().unwrap_or_default().into(),
            },
            other => other.to_string().into(),
        }
    }

    fn log_level(level: curia::Level) -> sentry::protocol::LogLevel {
        match level {
            curia::Level::Error => sentry::protocol::LogLevel::Error,
            curia::Level::Warn => sentry::protocol::LogLevel::Warn,
            curia::Level::Info => sentry::protocol::LogLevel::Info,
            curia::Level::Debug => sentry::protocol::LogLevel::Debug,
            curia::Level::Trace => sentry::protocol::LogLevel::Trace,
        }
    }

    fn breadcrumb_level(level: curia::Level) -> sentry::Level {
        match level {
            curia::Level::Error => sentry::Level::Error,
            curia::Level::Warn => sentry::Level::Warning,
            curia::Level::Info => sentry::Level::Info,
            curia::Level::Debug | curia::Level::Trace => sentry::Level::Debug,
        }
    }
}
