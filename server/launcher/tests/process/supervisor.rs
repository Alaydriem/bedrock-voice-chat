use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

use bvc_launcher::process::{
    BdsInput, ChildLock, IdleOutcome, Reaper, Supervisor, SupervisorEvent,
};

struct Harness;

impl Harness {
    const BVC_BODY: &'static str = "trap 'exit 0' TERM\nwhile true; do sleep 0.1; done";

    fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    // A supervisor with BVC running and the Reaper started, as the launcher has before BDS.
    fn with_bvc(dir: &Path, bvc_body: &str) -> (Supervisor, Sender<SupervisorEvent>, BdsInput) {
        let bvc = Self::script(dir, "bvc", bvc_body);
        let (events, receiver) = mpsc::channel();
        let child_lock = ChildLock::new();
        let input = BdsInput::new();
        let mut supervisor = Supervisor::new(receiver, input.clone(), child_lock.clone());
        supervisor.spawn_bvc(&bvc, dir).unwrap();
        Reaper::spawn(events.clone(), child_lock);
        (supervisor, events, input)
    }

    // A BDS running `bds_body` and a BVC that exits on SIGTERM.
    fn start(dir: &Path, bds_body: &str) -> (Supervisor, Sender<SupervisorEvent>) {
        let (supervisor, events, _input) = Self::start_with_input(dir, bds_body);
        (supervisor, events)
    }

    fn start_with_input(
        dir: &Path,
        bds_body: &str,
    ) -> (Supervisor, Sender<SupervisorEvent>, BdsInput) {
        let (mut supervisor, events, input) = Self::with_bvc(dir, Self::BVC_BODY);
        let bds = Self::script(dir, "bds", bds_body);
        supervisor.spawn_bds(&bds, dir, &[]).unwrap();
        (supervisor, events, input)
    }
}

const STOPS_ON_STOP: &str = r#"while read line; do [ "$line" = "stop" ] && exit 0; done"#;

#[test]
fn a_signal_stops_bds_with_the_stop_command_and_returns_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, events) = Harness::start(dir.path(), STOPS_ON_STOP);
    let started = Instant::now();

    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();

    assert_eq!(supervisor.run(), 0);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(!supervisor.has_children());
}

#[test]
fn bds_exiting_by_itself_stops_bvc_and_returns_the_bds_code() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, _events) = Harness::start(dir.path(), "sleep 0.2; exit 3");

    assert_eq!(supervisor.run(), 3);
    assert!(!supervisor.has_children());
}

#[test]
fn a_second_signal_kills_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, events) = Harness::start(
        dir.path(),
        "trap '' TERM\nwhile true; do sleep 0.1; done",
    );
    let started = Instant::now();

    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();
    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();

    assert_eq!(supervisor.run(), 128 + libc::SIGKILL);
    assert!(started.elapsed() < Duration::from_secs(8));
    assert!(!supervisor.has_children());
}

#[test]
fn bvc_exiting_leaves_bds_running() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, events, _input) = Harness::with_bvc(dir.path(), "exit 1");
    let bds = Harness::script(dir.path(), "bds", STOPS_ON_STOP);
    supervisor.spawn_bds(&bds, dir.path(), &[]).unwrap();

    assert_eq!(supervisor.idle(Duration::from_millis(500)), IdleOutcome::Elapsed);
    assert!(supervisor.has_children());
    assert!(supervisor.bvc_failed());

    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();
    assert_eq!(supervisor.run(), 0);
}

#[test]
fn a_bvc_stopped_by_a_signal_is_not_a_failure() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, events) = Harness::start(dir.path(), STOPS_ON_STOP);

    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();

    assert_eq!(supervisor.run(), 0);
    assert!(!supervisor.bvc_failed());
}

#[test]
fn a_bvc_stopped_after_bds_exits_is_not_a_failure() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, _events) = Harness::start(dir.path(), "sleep 0.2; exit 0");

    assert_eq!(supervisor.run(), 0);
    assert!(!supervisor.bvc_failed());
}

#[test]
fn a_bds_that_cannot_execute_is_an_error_while_the_reaper_runs() {
    let dir = tempfile::tempdir().unwrap();
    let not_executable = dir.path().join("bds");
    fs::write(&not_executable, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&not_executable, fs::Permissions::from_mode(0o644)).unwrap();
    let (mut supervisor, _events, _input) = Harness::with_bvc(dir.path(), Harness::BVC_BODY);

    // The race with the Reaper is narrow; enough attempts make it show without the lock.
    for _ in 0..2000 {
        assert!(supervisor.spawn_bds(&not_executable, dir.path(), &[]).is_err());
    }
    supervisor.stop_bvc();
}

#[test]
fn a_signal_queued_before_bds_starts_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, events, _input) = Harness::with_bvc(dir.path(), Harness::BVC_BODY);

    assert!(!supervisor.signal_pending());
    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();

    assert!(supervisor.signal_pending());
    supervisor.stop_bvc();
    assert!(!supervisor.has_children());
}

#[test]
fn a_bds_that_stops_reading_its_console_cannot_block_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, events, input) =
        Harness::start_with_input(dir.path(), "while true; do sleep 0.1; done");
    // More than a pipe buffer: the writer blocks once BDS stops reading.
    thread::spawn(move || input.write(&vec![b'x'; 1 << 20]));
    thread::sleep(Duration::from_millis(200));
    let started = Instant::now();

    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();
    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();

    assert_eq!(supervisor.run(), 128 + libc::SIGKILL);
    assert!(started.elapsed() < Duration::from_secs(8));
    assert!(!supervisor.has_children());
}

#[test]
fn a_restart_stop_leaves_bvc_running_and_bds_can_start_again() {
    let dir = tempfile::tempdir().unwrap();
    let (mut supervisor, events) = Harness::start(dir.path(), STOPS_ON_STOP);
    let bds = dir.path().join("bds");

    assert!(supervisor.restart_bds_stop());
    supervisor.spawn_bds(&bds, dir.path(), &[]).unwrap();
    assert_eq!(supervisor.idle(Duration::from_millis(200)), IdleOutcome::Elapsed);

    events.send(SupervisorEvent::Signal(libc::SIGTERM)).unwrap();
    assert_eq!(supervisor.run(), 0);
    assert!(!supervisor.has_children());
}
