use common::net::ReachabilityPlanner;
use common::structs::reachability::ReachabilityRequest;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn host_of_accepts_a_full_url() {
    assert_eq!(
        ReachabilityPlanner::host_of("https://bvc.example.com").unwrap(),
        "bvc.example.com"
    );
}

#[test]
fn host_of_accepts_a_bare_host() {
    assert_eq!(
        ReachabilityPlanner::host_of("bvc.example.com").unwrap(),
        "bvc.example.com"
    );
}

#[test]
fn host_of_drops_the_port_and_the_trailing_slash() {
    assert_eq!(
        ReachabilityPlanner::host_of("https://bvc.example.com:8443/").unwrap(),
        "bvc.example.com"
    );
}

// split(':') on an IPv6 literal yields "[", which is why this is parsed rather
// than string-split. Dual-stack makes it a live case.
#[test]
fn host_of_keeps_an_ipv6_literal_intact() {
    assert_eq!(
        ReachabilityPlanner::host_of("https://[::1]:443").unwrap(),
        "[::1]"
    );
}

#[test]
fn host_of_rejects_a_value_with_no_host() {
    assert!(ReachabilityPlanner::host_of("https://").is_err());
}

#[test]
fn ports_fall_back_to_the_default_when_a_server_advertises_nothing() {
    assert_eq!(ReachabilityPlanner::ports(&[], 0, None), vec![443]);
}

#[test]
fn ports_put_the_advertised_set_ahead_of_the_scalar() {
    assert_eq!(
        ReachabilityPlanner::ports(&[8443, 443], 443, None),
        vec![8443, 443]
    );
}

#[test]
fn request_carries_the_measured_https_port_rather_than_assuming_443() {
    let request = ReachabilityPlanner::request(
        "bvc.example.com".to_string(),
        vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))],
        vec![443],
        "https://bvc.example.com:8443",
        false,
    );

    assert_eq!(request.https_port, 8443);
}

#[test]
fn request_defaults_the_https_port_when_the_url_omits_it() {
    let request = ReachabilityPlanner::request(
        "bvc.example.com".to_string(),
        vec![IpAddr::V6(Ipv6Addr::LOCALHOST)],
        vec![443],
        "https://bvc.example.com",
        false,
    );

    assert_eq!(request.https_port, ReachabilityRequest::DEFAULT_HTTPS_PORT);
}

#[test]
fn request_points_the_https_probe_at_the_config_endpoint() {
    let request = ReachabilityPlanner::request(
        "bvc.example.com".to_string(),
        vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))],
        vec![443],
        "https://bvc.example.com/",
        false,
    );

    assert_eq!(request.https_url, "https://bvc.example.com/api/config");
}
