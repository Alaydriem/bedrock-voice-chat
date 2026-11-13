use common::structs::network::{EndpointSource, ServerAddress, ServerEndpoint};

#[test]
fn urls_carry_the_resolved_port() {
    let address = ServerAddress::parse("https://brave.example.com").unwrap();
    let endpoint = ServerEndpoint::new(address, 30001, EndpointSource::Srv);
    assert_eq!(endpoint.base_url(), "https://brave.example.com:30001");
    assert_eq!(endpoint.wss_url("/voice"), "wss://brave.example.com:30001/voice");
    assert_eq!(endpoint.tls_name(), "brave.example.com");
}

#[test]
fn an_ipv6_endpoint_brackets_its_urls() {
    let address = ServerAddress::parse("https://[2001:db8::1]").unwrap();
    let endpoint = ServerEndpoint::new(address, 8443, EndpointSource::Default);
    assert_eq!(endpoint.base_url(), "https://[2001:db8::1]:8443");
}
