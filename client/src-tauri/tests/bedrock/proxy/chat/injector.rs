use std::time::{Duration, Instant};

use bvc_client_lib::bedrock::ChatInjector;

#[test]
fn an_enqueued_line_reaches_the_session_loop_intact() {
    let injector = ChatInjector::new();
    let rx = injector.receiver();

    assert!(injector.enqueue("anyone got spare iron".to_string()));

    let pending = rx.try_recv().expect("enqueue should queue the line");
    assert_eq!(pending.text, "anyone got spare iron");
    assert!(!pending.is_expired(Instant::now()));
}

// A desktop app with no proxy session running still accepts sends. Without a bound it would
// accumulate them forever, so the queue is finite and a full queue reports rather than drops
// silently — the composer turns that `false` into a visible failure.
#[test]
fn a_full_queue_reports_failure_rather_than_swallowing_the_line() {
    let injector = ChatInjector::new();
    let _rx = injector.receiver();

    let mut accepted = 0;
    for i in 0..1000 {
        if injector.enqueue(format!("line {i}")) {
            accepted += 1;
        } else {
            break;
        }
    }

    assert!(accepted > 0, "the queue should accept something");
    assert!(accepted < 1000, "the queue must be bounded");
    assert!(
        !injector.enqueue("one more".to_string()),
        "a full queue must report failure"
    );
}

// A line that sat in the queue because no session was running is dropped rather than
// delivered late into a conversation that has moved on.
#[test]
fn a_pending_line_expires() {
    let injector = ChatInjector::new();
    let rx = injector.receiver();
    injector.enqueue("hello".to_string());

    let pending = rx.try_recv().expect("enqueue should queue the line");
    assert!(pending.is_expired(Instant::now() + Duration::from_secs(3600)));
}
