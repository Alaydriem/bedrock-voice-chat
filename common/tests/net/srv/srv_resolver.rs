use std::time::{Duration, Instant};

use common::net::srv::{SrvFallback, SrvResolver, SrvResolverConfig};
use common::structs::network::{EndpointSource, ServerAddress};

use super::{DnsResponder, DnsScript, DohResponder};

const PLAY_HOST: &str = "brave-otter.play.bedrockvc.stream";

struct Fixture;

impl Fixture {
    fn answer(port: u16) -> DnsScript {
        DnsScript::Answer {
            port,
            ttl: 60,
            target: None,
        }
    }

    fn resolver(nameservers: Vec<String>, fallback: SrvFallback) -> SrvResolver {
        SrvResolver::new(SrvResolverConfig {
            nameservers,
            fallback,
        })
    }

    fn address(stored: &str) -> ServerAddress {
        ServerAddress::parse(stored).expect("a parseable address")
    }
}

#[tokio::test]
async fn an_explicit_port_skips_every_lookup() {
    let authoritative = DnsResponder::start(Fixture::answer(1)).await;
    let doh = DohResponder::start(Fixture::answer(2)).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let resolution = resolver
        .resolve(&Fixture::address(
            "https://brave-otter.play.bedrockvc.stream:30001",
        ))
        .await;

    assert_eq!(resolution.endpoint.port(), 30001);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Explicit);
    assert_eq!(resolution.ttl, None);
    assert_eq!(authoritative.queries(), 0);
    assert_eq!(doh.queries(), 0);
}

#[tokio::test]
async fn an_ip_literal_without_a_port_is_443_without_a_lookup() {
    let authoritative = DnsResponder::start(Fixture::answer(1)).await;
    let doh = DohResponder::start(Fixture::answer(2)).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let resolution = resolver
        .resolve(&Fixture::address("https://203.0.113.9"))
        .await;

    assert_eq!(resolution.endpoint.port(), 443);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Default);
    assert_eq!(resolution.ttl, None);
    assert_eq!(authoritative.queries(), 0);
    assert_eq!(doh.queries(), 0);
}

#[tokio::test]
async fn a_play_name_uses_the_authoritative_answer() {
    let authoritative = DnsResponder::start(Fixture::answer(30001)).await;
    let doh = DohResponder::start(Fixture::answer(2)).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 30001);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Srv);
    assert_eq!(resolution.ttl, Some(Duration::from_secs(60)));
    assert_eq!(doh.queries(), 0);
}

#[tokio::test]
async fn an_authoritative_nxdomain_is_final() {
    let authoritative = DnsResponder::start(DnsScript::NxDomain).await;
    let doh = DohResponder::start(Fixture::answer(2)).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 443);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Default);
    assert_eq!(doh.queries(), 0);
}

#[tokio::test]
async fn the_next_nameserver_is_asked_when_one_is_silent() {
    let silent = DnsResponder::start(DnsScript::Silent).await;
    let answering = DnsResponder::start(Fixture::answer(30003)).await;
    let resolver = Fixture::resolver(
        vec![silent.nameserver(), answering.nameserver()],
        SrvFallback::None,
    );

    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 30003);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Srv);
    assert_eq!(silent.queries(), 1);
    assert_eq!(answering.queries(), 1);
}

#[tokio::test]
async fn a_truncated_udp_answer_is_retried_over_tcp() {
    let authoritative = DnsResponder::start(DnsScript::TruncatedUdp {
        port: 30004,
        ttl: 60,
    })
    .await;
    let resolver = Fixture::resolver(vec![authoritative.nameserver()], SrvFallback::None);

    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 30004);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Srv);
    assert_eq!(authoritative.queries(), 2);
}

#[tokio::test]
async fn a_silent_authoritative_server_falls_back_to_https() {
    let authoritative = DnsResponder::start(DnsScript::Silent).await;
    let doh = DohResponder::start(Fixture::answer(30002)).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 30002);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Srv);
    assert_eq!(doh.queries(), 1);
}

#[tokio::test]
async fn another_host_skips_the_authoritative_servers() {
    let authoritative = DnsResponder::start(Fixture::answer(1)).await;
    let doh = DohResponder::start(Fixture::answer(30005)).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let resolution = resolver
        .resolve(&Fixture::address("https://brave.example.com"))
        .await;

    assert_eq!(resolution.endpoint.port(), 30005);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Srv);
    assert_eq!(authoritative.queries(), 0);
}

#[tokio::test]
async fn every_lookup_failing_gives_443_within_the_timeouts() {
    let authoritative = DnsResponder::start(DnsScript::Silent).await;
    let doh = DohResponder::start(DnsScript::Silent).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let started = Instant::now();
    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 443);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Default);
    assert!(started.elapsed() < Duration::from_secs(6));
}

#[tokio::test]
async fn an_authoritative_empty_answer_is_final() {
    let authoritative = DnsResponder::start(DnsScript::Empty).await;
    let doh = DohResponder::start(Fixture::answer(2)).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 443);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Default);
    assert_eq!(doh.queries(), 0);
}

#[tokio::test]
async fn a_hanging_https_fallback_is_bounded() {
    let authoritative = DnsResponder::start(DnsScript::Silent).await;
    let doh = DohResponder::start(DnsScript::Hang).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let started = Instant::now();
    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 443);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Default);
    assert_eq!(doh.queries(), 1);
    assert!(started.elapsed() < Duration::from_secs(6));
}

#[tokio::test]
async fn an_answer_pointing_at_another_host_gives_443() {
    let authoritative = DnsResponder::start(DnsScript::Answer {
        port: 30001,
        ttl: 60,
        target: Some("other.example.com.".to_string()),
    })
    .await;
    let doh = DohResponder::start(Fixture::answer(2)).await;
    let resolver = Fixture::resolver(
        vec![authoritative.nameserver()],
        SrvFallback::Https(doh.url()),
    );

    let resolution = resolver
        .resolve(&Fixture::address(&format!("https://{PLAY_HOST}")))
        .await;

    assert_eq!(resolution.endpoint.port(), 443);
    assert_eq!(resolution.endpoint.source(), EndpointSource::Default);
    assert_eq!(doh.queries(), 0);
}
