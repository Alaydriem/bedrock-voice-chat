use std::time::{Duration, Instant};

use bvc_client_lib::bedrock::QueryStateInjector;

#[test]
fn enqueues_encoded_message_for_the_session_loop() {
    let injector = QueryStateInjector::new();
    let rx = injector.receiver();

    injector.enqueue("!bvcs:1:q:m=1;d=0;r=0;g=-".to_string());

    let pending = rx.try_recv().expect("enqueue should queue the message");
    assert_eq!(pending.message, "!bvcs:1:q:m=1;d=0;r=0;g=-");
    assert!(!pending.is_expired(Instant::now()));
}

#[test]
fn pending_message_expires_after_its_ttl() {
    let injector = QueryStateInjector::new();
    let rx = injector.receiver();

    injector.enqueue("!bvcs:2:q:m=0;d=0;r=0;g=-".to_string());

    let pending = rx.try_recv().expect("enqueue should queue the message");
    // Stale state must not be injected long after the fact; the deadline is
    // bounded, so a point far in the future is past it.
    assert!(pending.is_expired(Instant::now() + Duration::from_secs(60)));
}

#[test]
fn a_full_queue_drops_instead_of_blocking() {
    let injector = QueryStateInjector::new();
    // No consumer: fill past any reasonable bound; enqueue must never block or
    // grow unboundedly on a desktop app that never starts the proxy.
    for i in 0..10_000 {
        injector.enqueue(format!("!bvcs:{i}:q:m=0;d=0;r=0;g=-"));
    }
    let rx = injector.receiver();
    let mut drained = 0;
    while rx.try_recv().is_ok() {
        drained += 1;
    }
    assert!(
        drained < 10_000,
        "queue must be bounded (drained {drained} of 10000)"
    );
}
