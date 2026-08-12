use async_trait::async_trait;
use bvc_client_lib::audio::recording::{ExportRun, TrackSink};
use common::structs::recording::{RecordingTrack, TrackKind};
use std::sync::Mutex;

fn track(display: &str) -> RecordingTrack {
    RecordingTrack {
        keys: vec![format!("minecraft:{display}")],
        display: display.to_string(),
        kind: TrackKind::Player,
    }
}

/// A sink that fails on the tracks it is told to and records everything it was asked for.
struct FakeSink {
    fails: Vec<String>,
    seen: Mutex<Vec<String>>,
    progress: Mutex<Vec<(String, u32, u32)>>,
}

#[async_trait]
impl TrackSink for FakeSink {
    async fn write(&self, track: &RecordingTrack) -> Result<(), anyhow::Error> {
        self.seen.lock().unwrap().push(track.display.clone());
        if self.fails.contains(&track.display) {
            return Err(anyhow::anyhow!("no such file"));
        }
        Ok(())
    }

    fn progressed(&self, track: &RecordingTrack, index: u32, total: u32) {
        self.progress
            .lock()
            .unwrap()
            .push((track.display.clone(), index, total));
    }
}

fn sink(fails: &[&str]) -> FakeSink {
    FakeSink {
        fails: fails.iter().map(|f| f.to_string()).collect(),
        seen: Mutex::new(Vec::new()),
        progress: Mutex::new(Vec::new()),
    }
}

// The defect this replaces returned success no matter what happened inside the loop.
#[tokio::test]
async fn a_track_that_fails_is_reported_by_name() {
    let sink = sink(&["Petra"]);
    let outcome = ExportRun::execute(&[track("Alaydriem"), track("Petra")], &sink).await;

    assert_eq!(outcome.written, vec!["Alaydriem".to_string()]);
    assert_eq!(outcome.failed.len(), 1);
    assert_eq!(outcome.failed[0].track, "Petra");
}

// One unreadable track must not cost somebody the other six.
#[tokio::test]
async fn a_failure_does_not_stop_the_tracks_after_it() {
    let sink = sink(&["Petra"]);
    let outcome =
        ExportRun::execute(&[track("Alaydriem"), track("Petra"), track("Juno")], &sink).await;

    assert_eq!(sink.seen.lock().unwrap().len(), 3);
    assert_eq!(
        outcome.written,
        vec!["Alaydriem".to_string(), "Juno".to_string()]
    );
}

#[tokio::test]
async fn every_track_reports_progress_against_the_same_total() {
    let sink = sink(&[]);
    ExportRun::execute(&[track("Alaydriem"), track("Petra")], &sink).await;

    assert_eq!(
        *sink.progress.lock().unwrap(),
        vec![("Alaydriem".to_string(), 1, 2), ("Petra".to_string(), 2, 2)]
    );
}

#[tokio::test]
async fn exporting_nothing_is_an_empty_outcome_and_not_an_error() {
    let sink = sink(&[]);
    let outcome = ExportRun::execute(&[], &sink).await;

    assert!(outcome.written.is_empty());
    assert!(outcome.failed.is_empty());
}
