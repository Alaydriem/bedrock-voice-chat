use std::sync::{Arc, Mutex};
use std::time::Duration;

use bvc_client_lib::groups::protected::{
    GroupKeyService, GroupPassword, KeySocketEvent, LostReason, ProtectedGroupEvent,
    ProtectedGroupRunner, SealGate, SystemClock,
};
use tokio::sync::{mpsc, oneshot};

fn joiner(gate: Arc<SealGate>) -> (GroupKeyService<SystemClock>, mpsc::Receiver<ProtectedGroupEvent>) {
    let (outbound, _outbound_rx) = mpsc::channel(16);
    let (events, events_rx) = mpsc::channel(16);
    let service = GroupKeyService::join(
        "ABCD2345".to_string(),
        "minecraft:Bob".parse().unwrap(),
        GroupPassword::new("correct horse").unwrap(),
        SystemClock,
        outbound,
        events,
        gate,
    )
    .unwrap();
    (service, events_rx)
}

#[tokio::test]
async fn a_closed_socket_ends_the_runner_with_its_reason() {
    let gate = Arc::new(SealGate::new());
    let (service, service_events) = joiner(gate.clone());
    let (socket_tx, socket_events) = mpsc::channel(4);
    let (_stop_tx, stop) = oneshot::channel();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();

    socket_tx
        .send(KeySocketEvent::Closed(Some("NoAdmitterOnline".to_string())))
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(5),
        ProtectedGroupRunner::run(service, socket_events, service_events, stop, move |event| {
            record.lock().unwrap().push(event)
        }),
    )
    .await
    .expect("the runner returns once the socket closes");

    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[ProtectedGroupEvent::Lost(LostReason::NoAdmitter)]
    );
    assert!(gate.is_protected());
    assert!(!gate.has_keys());
    // Losing the group is silence, never plaintext: the next microphone frame is dropped.
    assert_eq!(
        bvc_client_lib::audio::stream::protected_frame_filter::ProtectedFrameFilter::new(gate)
            .outbound(&[0x11; 80]),
        bvc_client_lib::audio::stream::protected_frame_filter::OutboundDecision::Drop
    );
}

#[tokio::test]
async fn stopping_the_runner_returns_without_a_lost_event() {
    let gate = Arc::new(SealGate::new());
    let (service, service_events) = joiner(gate);
    let (_socket_tx, socket_events) = mpsc::channel(4);
    let (stop_tx, stop) = oneshot::channel();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();

    stop_tx.send(()).unwrap();
    tokio::time::timeout(
        Duration::from_secs(5),
        ProtectedGroupRunner::run(service, socket_events, service_events, stop, move |event| {
            record.lock().unwrap().push(event)
        }),
    )
    .await
    .expect("the runner returns once stopped");

    assert!(seen.lock().unwrap().is_empty());
}
