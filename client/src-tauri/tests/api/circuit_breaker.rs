use bvc_client_lib::EndpointBreaker;

// Each test owns its endpoint key: breakers live in a process-global registry.
fn breaker(name: &str) -> std::sync::Arc<EndpointBreaker> {
    EndpointBreaker::for_endpoint(name)
}

// The whole point of the change: a client pointed at a dead host reports the
// outage once, not once per cooldown probe for as long as it runs.
#[test]
fn only_the_first_failure_of_an_outage_is_reported() {
    let breaker = breaker("first-failure-only");

    assert!(breaker.on_transport_failure().first);

    for _ in 0..10 {
        assert!(!breaker.on_transport_failure().first);
    }
}

#[test]
fn reachability_re_arms_the_report_for_the_next_outage() {
    let breaker = breaker("re-arms-after-success");

    assert!(breaker.on_transport_failure().first);
    assert!(!breaker.on_transport_failure().first);

    breaker.on_success();

    assert!(breaker.on_transport_failure().first);
}

// Opening is independent of reporting: the fifth consecutive failure still
// short-circuits later requests even though it is no longer worth an error.
#[test]
fn the_threshold_failure_opens_without_being_reported() {
    let breaker = breaker("opens-unreported");

    assert!(breaker.on_transport_failure().first);
    for _ in 0..3 {
        assert!(!breaker.on_transport_failure().opened);
    }

    let report = breaker.on_transport_failure();
    assert!(report.opened);
    assert!(!report.first);
    assert!(!breaker.allow());
}

// A half-open probe that fails re-opens the breaker, and that re-open is not a
// fresh outage — it is the same one still running.
#[test]
fn a_failed_half_open_probe_re_opens_without_reporting() {
    let breaker = breaker("half-open-probe");

    for _ in 0..5 {
        breaker.on_transport_failure();
    }
    assert!(!breaker.allow());

    breaker.on_success();
    for _ in 0..5 {
        breaker.on_transport_failure();
    }

    let report = breaker.on_transport_failure();
    assert!(!report.first);
}
