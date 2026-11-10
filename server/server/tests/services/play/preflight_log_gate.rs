use bvc_server_lib::services::play::PreflightLogGate;
use common::structs::relay::enroll::PlayPreflightOutcome;

struct Outcomes;

impl Outcomes {
    fn timed_out() -> PlayPreflightOutcome {
        PlayPreflightOutcome::TimedOut {
            address: "203.0.113.9".to_string(),
            port: 443,
        }
    }

    fn refused() -> PlayPreflightOutcome {
        PlayPreflightOutcome::Refused {
            address: "203.0.113.9".to_string(),
            port: 443,
        }
    }
}

#[test]
fn a_repeated_outcome_is_logged_once() {
    let mut gate = PreflightLogGate::new();

    assert!(gate.should_log(&Outcomes::timed_out()));
    assert!(!gate.should_log(&Outcomes::timed_out()));
}

// A rate limit between two identical failures is not a change: the operator sees the failure
// once, and never sees the rate limit as a failure of its own.
#[test]
fn a_rate_limit_between_identical_failures_logs_the_failure_once() {
    let mut gate = PreflightLogGate::new();
    let limited = PlayPreflightOutcome::RateLimited { retry_after_secs: 5 };

    assert!(gate.should_log(&Outcomes::refused()));
    assert!(!gate.should_log(&limited));
    assert!(!gate.should_log(&Outcomes::refused()));
}

#[test]
fn a_different_outcome_is_logged_again() {
    let mut gate = PreflightLogGate::new();

    assert!(gate.should_log(&Outcomes::timed_out()));
    assert!(gate.should_log(&Outcomes::refused()));
    assert!(gate.should_log(&Outcomes::timed_out()));
}
