use std::net::IpAddr;
use std::time::Duration;

use bvc_relay_service::play::{PlayPreflightLimiter, PlayPreflightService};
use bvc_relay_service::validation::AddressProbe;
use common::structs::relay::enroll::PlayPreflightOutcome;

const PORT: u16 = 443;

struct Fixture;

impl Fixture {
    fn address(text: &str) -> IpAddr {
        text.parse().expect("an IP address")
    }

    fn service(answers: bool, per_address_per_hour: u32) -> PlayPreflightService {
        Self::service_with(
            answers,
            per_address_per_hour,
            1,
            PlayPreflightService::MAX_CONCURRENT_PROBES,
        )
    }

    fn service_with(
        answers: bool,
        per_address_per_hour: u32,
        delay_ms: u64,
        max_concurrent_probes: usize,
    ) -> PlayPreflightService {
        PlayPreflightService::new(
            AddressProbe::Fixed(answers),
            vec![Duration::from_millis(delay_ms)],
            PlayPreflightLimiter::new(Duration::from_millis(1), per_address_per_hour),
            false,
            max_concurrent_probes,
            Vec::new(),
        )
    }

    fn service_with_own_addresses(own: Vec<IpAddr>) -> PlayPreflightService {
        PlayPreflightService::new(
            AddressProbe::Fixed(true),
            vec![Duration::from_millis(1)],
            PlayPreflightLimiter::new(Duration::from_millis(1), 30),
            false,
            PlayPreflightService::MAX_CONCURRENT_PROBES,
            own,
        )
    }
}

#[tokio::test]
async fn the_registrys_own_address_is_refused_as_a_candidate() {
    let service = Fixture::service_with_own_addresses(vec![Fixture::address("198.51.100.10"), Fixture::address("8.8.4.4")]);

    for own in ["8.8.4.4", "::ffff:8.8.4.4"] {
        let outcome = service
            .prepare("node-a", Fixture::address(own), None, PORT)
            .expect_err("the registry's own address is refused");

        assert!(
            matches!(outcome, PlayPreflightOutcome::AddressNotPublic { .. }),
            "{outcome:?}"
        );
    }
    assert!(service.prepare("node-a", Fixture::address("8.8.8.8"), None, PORT).is_ok());
}

#[tokio::test]
async fn one_requester_prefix_cannot_hold_more_than_its_share_of_probes() {
    let service = Fixture::service_with(true, 30, 300, PlayPreflightService::MAX_CONCURRENT_PROBES);
    let observed = Some(Fixture::address("203.0.113.10"));
    let ticket = |node: &str, target: &str| {
        service
            .prepare(node, Fixture::address(target), observed, PORT)
            .expect("prepared")
    };
    let (a, b, c) = (
        ticket("node-a", "8.8.8.8"),
        ticket("node-b", "1.1.1.1"),
        ticket("node-c", "9.9.9.9"),
    );

    let (a, b, c) = tokio::join!(
        service.run("node-a", &a),
        service.run("node-b", &b),
        service.run("node-c", &c)
    );

    assert!(matches!(a, PlayPreflightOutcome::Reachable { .. }), "{a:?}");
    assert!(matches!(b, PlayPreflightOutcome::Reachable { .. }), "{b:?}");
    assert!(
        matches!(c, PlayPreflightOutcome::RateLimited { retry_after_secs } if retry_after_secs > 0),
        "{c:?}"
    );

    let later = service
        .prepare("node-d", Fixture::address("4.4.4.4"), observed, PORT)
        .expect("prepared");
    assert!(matches!(
        service.run("node-d", &later).await,
        PlayPreflightOutcome::Reachable { .. }
    ));
}

#[tokio::test]
async fn a_private_candidate_is_refused_without_a_probe() {
    let service = Fixture::service(true, 30);

    let outcome = service
        .prepare("node-a", Fixture::address("10.0.0.5"), None, PORT)
        .expect_err("a private address is refused");

    assert_eq!(
        outcome,
        PlayPreflightOutcome::AddressNotPublic {
            address: "10.0.0.5".to_string()
        }
    );
    assert!(!service.has_passed("node-a"));
}

#[tokio::test]
async fn a_cgnat_candidate_is_refused_without_a_probe() {
    let outcome = Fixture::service(true, 30)
        .prepare("node-a", Fixture::address("100.64.1.1"), None, PORT)
        .expect_err("a CGNAT address is refused");

    assert_eq!(
        outcome,
        PlayPreflightOutcome::AddressCgnat {
            address: "100.64.1.1".to_string()
        }
    );
}

#[tokio::test]
async fn a_reachable_candidate_passes_and_is_remembered() {
    let service = Fixture::service(true, 30);
    let ticket = service
        .prepare("node-a", Fixture::address("8.8.8.8"), None, PORT)
        .expect("a public address is prepared");

    let outcome = service.run("node-a", &ticket).await;

    assert_eq!(
        outcome,
        PlayPreflightOutcome::Reachable {
            address: "8.8.8.8".to_string(),
            port: PORT
        }
    );
    assert!(service.has_passed("node-a"));
    assert!(!service.has_passed("node-b"));
}

#[tokio::test]
async fn a_failed_probe_is_not_remembered() {
    let service = Fixture::service(false, 30);
    let ticket = service
        .prepare("node-a", Fixture::address("8.8.8.8"), None, PORT)
        .expect("a public address is prepared");

    let outcome = service.run("node-a", &ticket).await;

    assert_eq!(
        outcome,
        PlayPreflightOutcome::WrongService {
            address: "8.8.8.8".to_string(),
            port: PORT
        }
    );
    assert!(!service.has_passed("node-a"));
}

#[tokio::test]
async fn many_nodes_from_one_address_are_rate_limited_without_probing() {
    let service = Fixture::service(true, PlayPreflightLimiter::PER_ADDRESS_PER_HOUR);

    for node in 0..PlayPreflightLimiter::PER_ADDRESS_PER_HOUR {
        service
            .prepare(&format!("node-{node}"), Fixture::address("8.8.8.8"), None, PORT)
            .expect("within the allowance");
    }
    let outcome = service
        .prepare("node-extra", Fixture::address("8.8.8.8"), None, PORT)
        .expect_err("over the allowance");

    assert!(
        matches!(outcome, PlayPreflightOutcome::RateLimited { retry_after_secs } if retry_after_secs > 0),
        "{outcome:?}"
    );
    assert!(!service.has_passed("node-extra"));
}

// A requester that changes the candidate each time still arrives from one address.
#[tokio::test]
async fn many_candidates_from_one_observed_address_are_rate_limited_without_probing() {
    let service = Fixture::service(true, PlayPreflightLimiter::PER_ADDRESS_PER_HOUR);
    let observed = Some(Fixture::address("203.0.113.77"));

    for node in 0..PlayPreflightLimiter::PER_ADDRESS_PER_HOUR {
        service
            .prepare(
                &format!("node-{node}"),
                Fixture::address(&format!("8.8.{}.{}", node / 200, node % 200 + 1)),
                observed,
                PORT,
            )
            .expect("within the allowance");
    }
    let outcome = service
        .prepare("node-extra", Fixture::address("9.9.9.9"), observed, PORT)
        .expect_err("over the allowance");

    assert!(
        matches!(outcome, PlayPreflightOutcome::RateLimited { retry_after_secs } if retry_after_secs > 0),
        "{outcome:?}"
    );
    assert!(!service.has_passed("node-extra"));
}

// The probes in flight are capped: a caller over the cap is refused, not queued.
#[tokio::test]
async fn a_probe_over_the_concurrency_cap_is_refused_not_queued() {
    let service = Fixture::service_with(true, 30, 300, 1);
    let first = service
        .prepare("node-a", Fixture::address("8.8.8.8"), None, PORT)
        .expect("prepared");
    let second = service
        .prepare("node-b", Fixture::address("1.1.1.1"), None, PORT)
        .expect("prepared");

    let (first, second) = tokio::join!(service.run("node-a", &first), service.run("node-b", &second));

    assert!(matches!(first, PlayPreflightOutcome::Reachable { .. }), "{first:?}");
    assert!(
        matches!(second, PlayPreflightOutcome::RateLimited { retry_after_secs } if retry_after_secs > 0),
        "{second:?}"
    );
    assert!(service.has_passed("node-a"));
    assert!(!service.has_passed("node-b"));

    let later = service
        .prepare("node-c", Fixture::address("9.9.9.9"), None, PORT)
        .expect("prepared");
    assert!(matches!(
        service.run("node-c", &later).await,
        PlayPreflightOutcome::Reachable { .. }
    ));
}
