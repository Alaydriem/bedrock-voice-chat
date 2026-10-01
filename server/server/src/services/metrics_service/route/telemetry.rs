use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use common::curia;
use metrics::{Counter, Histogram, counter, histogram};
use tokio::sync::mpsc;

use super::RouteFrameReport;
use crate::services::metrics_service::interaction::{InteractionTracker, RouteRejection};
use crate::services::metrics_service::metric::Metric;

/// Carries routing measurements from the audio path to the metrics recorder.
///
/// The routing thread pays for one `try_send` per frame. Counters, the duration histogram and
/// the interaction tracker are all written by `flush`, which the metrics service runs on its
/// own task. Nothing here is real time: a count lands at most one flush interval late.
///
/// Lossy under overload by design. A full queue drops the report rather than stall a frame,
/// and the next flush says how many were lost.
pub struct RouteTelemetry {
    tx: mpsc::Sender<RouteFrameReport>,
    // A `Mutex` only so `flush` can take `&self`; the routing path never locks it.
    rx: Mutex<mpsc::Receiver<RouteFrameReport>>,
    dropped: AtomicU64,
    frames_routed: Counter,
    route_duration: Histogram,
    queue_full_drops: Counter,
    // One resolved handle per reason, so neither a flush nor a single out-of-loop rejection
    // builds a key and searches the registry.
    rejections: [Counter; RouteRejection::ALL.len()],
}

impl RouteTelemetry {
    // A second of frames for ~300 continuous speakers at the one-second flush interval.
    const CAPACITY: usize = 16_384;

    // Resolves every handle against the installed recorder, so it must be built after the
    // global recorder is set.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(Self::CAPACITY);
        Self {
            tx,
            rx: Mutex::new(rx),
            dropped: AtomicU64::new(0),
            frames_routed: counter!(Metric::AudioFramesRoutedTotal.name()),
            route_duration: histogram!(Metric::AudioRouteDurationSeconds.name()),
            queue_full_drops: counter!(Metric::AudioRouteRecipientDropsTotal.name()),
            rejections: std::array::from_fn(|i| {
                counter!(
                    Metric::AudioRouteRejectionsTotal.name(),
                    "reason" => RouteRejection::ALL[i].label()
                )
            }),
        }
    }

    pub fn submit(&self, report: RouteFrameReport) {
        if self.tx.try_send(report).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    // For a whole-frame rejection outside the fan-out loop, which has no report to join.
    pub fn reject_now(&self, reason: RouteRejection) {
        self.rejections[reason.index()].increment(1);
    }

    /// Applies every queued report. Sums locally and writes each counter once per flush, so the
    /// recorder sees one increment per reason however many frames were queued.
    pub fn flush(&self, interactions: &InteractionTracker) {
        let mut frames = 0u64;
        let mut rejections = [0u64; RouteRejection::ALL.len()];
        let mut queue_full_drops = 0u64;

        {
            let mut rx = self.rx.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            while let Ok(report) = rx.try_recv() {
                frames += 1;
                self.route_duration.record(report.duration().as_secs_f64());
                for (reason, count) in report.rejections() {
                    rejections[reason.index()] += u64::from(count);
                }
                queue_full_drops += u64::from(report.queue_full_drops());

                if let Some(sender) = report.sender() {
                    for (route, recipient) in report.deliveries() {
                        interactions.record_delivery(*route, sender, *recipient);
                    }
                }
            }
        }

        if frames > 0 {
            self.frames_routed.increment(frames);
        }
        for (handle, count) in self.rejections.iter().zip(rejections) {
            if count > 0 {
                handle.increment(count);
            }
        }
        if queue_full_drops > 0 {
            self.queue_full_drops.increment(queue_full_drops);
        }

        let dropped = self.dropped.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            curia::warn!("route telemetry queue full; reports dropped", { "dropped": dropped });
        }
    }
}

impl Default for RouteTelemetry {
    fn default() -> Self {
        Self::new()
    }
}
