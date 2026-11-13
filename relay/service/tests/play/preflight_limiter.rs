use std::net::IpAddr;
use std::time::Duration;

use bvc_relay_service::play::PlayPreflightLimiter;

struct Addresses;

impl Addresses {
    fn parse(text: &str) -> IpAddr {
        text.parse().expect("an IP address")
    }
}

#[test]
fn a_second_preflight_inside_the_node_window_is_refused_with_its_wait() {
    let limiter = PlayPreflightLimiter::new(Duration::from_secs(60), 30);

    limiter
        .admit("node-a", Addresses::parse("8.8.8.8"), None)
        .expect("the first is admitted");
    let wait = limiter
        .admit("node-a", Addresses::parse("8.8.8.8"), None)
        .expect_err("the second is refused");

    assert!(wait > Duration::from_secs(50) && wait <= Duration::from_secs(60), "{wait:?}");
    limiter
        .admit("node-b", Addresses::parse("1.1.1.1"), None)
        .expect("another node is unaffected");
}

#[test]
fn ipv6_addresses_in_one_64_share_an_allowance() {
    let limiter = PlayPreflightLimiter::new(Duration::from_millis(1), 2);

    limiter.admit("node-a", Addresses::parse("2606:4700:1:2::1"), None).expect("admitted");
    limiter.admit("node-b", Addresses::parse("2606:4700:1:2::ffff"), None).expect("admitted");
    assert!(limiter.admit("node-c", Addresses::parse("2606:4700:1:2:3::1"), None).is_err());

    limiter
        .admit("node-d", Addresses::parse("2606:4700:1:3::1"), None)
        .expect("another /64 has its own allowance");
}

#[test]
fn observed_ipv6_addresses_in_one_56_share_an_allowance() {
    let limiter = PlayPreflightLimiter::new(Duration::from_millis(1), 2);

    limiter
        .admit("node-a", Addresses::parse("8.8.8.8"), Some(Addresses::parse("2606:4700:1:200::1")))
        .expect("admitted");
    limiter
        .admit("node-b", Addresses::parse("1.1.1.1"), Some(Addresses::parse("2606:4700:1:2ff::1")))
        .expect("admitted");
    assert!(
        limiter
            .admit("node-c", Addresses::parse("9.9.9.9"), Some(Addresses::parse("2606:4700:1:244::1")))
            .is_err()
    );

    limiter
        .admit("node-d", Addresses::parse("9.9.9.9"), Some(Addresses::parse("2606:4700:1:300::1")))
        .expect("another /56 has its own allowance");
}

#[test]
fn an_ipv4_mapped_address_shares_the_allowance_of_its_ipv4_address() {
    let limiter = PlayPreflightLimiter::new(Duration::from_millis(1), 1);

    limiter.admit("node-a", Addresses::parse("8.8.8.8"), None).expect("admitted");
    assert!(limiter.admit("node-b", Addresses::parse("::ffff:8.8.8.8"), None).is_err());
}

#[test]
fn the_observed_address_has_its_own_allowance_whatever_the_candidate() {
    let limiter = PlayPreflightLimiter::new(Duration::from_millis(1), 2);
    let observed = Some(Addresses::parse("203.0.113.7"));

    limiter.admit("node-a", Addresses::parse("8.8.8.8"), observed).expect("admitted");
    limiter.admit("node-b", Addresses::parse("1.1.1.1"), observed).expect("admitted");
    assert!(limiter.admit("node-c", Addresses::parse("9.9.9.9"), observed).is_err());

    limiter
        .admit("node-d", Addresses::parse("9.9.9.9"), Some(Addresses::parse("203.0.113.8")))
        .expect("another observed address has its own allowance");
}

#[test]
fn a_refusal_on_the_observed_address_does_not_spend_the_candidates_allowance() {
    let limiter = PlayPreflightLimiter::new(Duration::from_millis(1), 1);
    let observed = Some(Addresses::parse("203.0.113.7"));

    limiter.admit("node-a", Addresses::parse("8.8.8.8"), observed).expect("admitted");
    assert!(limiter.admit("node-b", Addresses::parse("1.1.1.1"), observed).is_err());

    limiter
        .admit("node-c", Addresses::parse("1.1.1.1"), Some(Addresses::parse("203.0.113.9")))
        .expect("the refused request did not count against 1.1.1.1");
}

#[test]
fn a_candidate_equal_to_the_observed_address_is_counted_once() {
    let limiter = PlayPreflightLimiter::new(Duration::from_millis(1), 2);
    let same = Addresses::parse("8.8.8.8");

    limiter.admit("node-a", same, Some(same)).expect("admitted");
    limiter.admit("node-b", same, Some(same)).expect("admitted");
    assert!(limiter.admit("node-c", same, Some(same)).is_err());
}
