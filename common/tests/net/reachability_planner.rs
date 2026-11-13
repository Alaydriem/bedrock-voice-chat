use common::net::ReachabilityPlanner;
use common::structs::network::{EndpointSource, ServerAddress, ServerEndpoint};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

struct EndpointFixture;

impl EndpointFixture {
    fn at(identity: &str, port: u16, source: EndpointSource) -> ServerEndpoint {
        ServerEndpoint::new(ServerAddress::parse(identity).unwrap(), port, source)
    }
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
fn request_takes_the_https_port_from_the_endpoint() {
    let endpoint = EndpointFixture::at("https://brave.example.com", 30001, EndpointSource::Srv);
    let request = ReachabilityPlanner::request(
        vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))],
        vec![443],
        &endpoint,
        false,
    );

    assert_eq!(request.https_port, 30001);
    assert_eq!(
        request.https_url,
        "https://brave.example.com:30001/api/config"
    );
    assert_eq!(request.host, "brave.example.com");
}

#[test]
fn request_keeps_an_ipv6_host_bracketed() {
    let endpoint = EndpointFixture::at("https://[::1]:8443", 8443, EndpointSource::Explicit);
    let request = ReachabilityPlanner::request(
        vec![IpAddr::V6(Ipv6Addr::LOCALHOST)],
        vec![443],
        &endpoint,
        false,
    );

    assert_eq!(request.host, "[::1]");
    assert_eq!(request.https_url, "https://[::1]:8443/api/config");
}
