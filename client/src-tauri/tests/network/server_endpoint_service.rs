use std::time::{Duration, Instant};

use bvc_client_lib::network::{CachedEndpoint, ServerEndpointExpiry, ServerEndpointService};
use moka::Expiry;
use common::net::srv::{SrvFallback, SrvResolution, SrvResolver, SrvResolverConfig};
use common::structs::network::{EndpointSource, ServerAddress, ServerEndpoint};

use super::{DnsResponder, DnsScript};

const PLAY_SERVER: &str = "https://brave-otter.play.bedrockvc.stream";

struct Fixture;

impl Fixture {
    fn lifetime(source: EndpointSource, ttl: Option<Duration>) -> Duration {
        let address = ServerAddress::parse(PLAY_SERVER).expect("a parseable address");
        ServerEndpointService::lifetime(&SrvResolution {
            endpoint: ServerEndpoint::new(address, 30001, source),
            ttl,
        })
    }

    fn service(responder: &DnsResponder) -> ServerEndpointService {
        ServerEndpointService::new(SrvResolver::new(SrvResolverConfig {
            nameservers: vec![responder.nameserver()],
            fallback: SrvFallback::None,
        }))
    }
}

#[tokio::test]
async fn resolve_within_the_ttl_uses_the_cache() {
    let responder = DnsResponder::start(DnsScript::Answer {
        port: 30001,
        ttl: 60,
    })
    .await;
    let service = Fixture::service(&responder);

    let first = service.resolve(PLAY_SERVER).await.expect("an endpoint");
    responder.answer_with(DnsScript::Answer {
        port: 30002,
        ttl: 60,
    });
    let second = service.resolve(PLAY_SERVER).await.expect("an endpoint");

    assert_eq!(first.port(), 30001);
    assert_eq!(second.port(), 30001);
    assert_eq!(responder.queries(), 1);
}

#[tokio::test]
async fn refresh_sees_a_changed_port() {
    let responder = DnsResponder::start(DnsScript::Answer {
        port: 30001,
        ttl: 60,
    })
    .await;
    let service = Fixture::service(&responder);

    let first = service.resolve(PLAY_SERVER).await.expect("an endpoint");
    responder.answer_with(DnsScript::Answer {
        port: 30002,
        ttl: 60,
    });
    let refreshed = service.refresh(PLAY_SERVER).await.expect("an endpoint");
    let after = service.resolve(PLAY_SERVER).await.expect("an endpoint");

    assert_eq!(first.port(), 30001);
    assert_eq!(refreshed.port(), 30002);
    assert_eq!(after.port(), 30002);
}

#[tokio::test]
async fn a_default_result_is_cached_briefly() {
    let responder = DnsResponder::start(DnsScript::NxDomain).await;
    let service = Fixture::service(&responder);

    let first = service.resolve(PLAY_SERVER).await.expect("an endpoint");
    let queries_after_first = responder.queries();
    let second = service.resolve(PLAY_SERVER).await.expect("an endpoint");

    assert_eq!(first.port(), 443);
    assert_eq!(first.source(), EndpointSource::Default);
    assert_eq!(second.port(), 443);
    assert_eq!(responder.queries(), queries_after_first);
}

#[test]
fn a_short_record_ttl_is_raised_to_thirty_seconds() {
    assert_eq!(
        Fixture::lifetime(EndpointSource::Srv, Some(Duration::from_secs(5))),
        Duration::from_secs(30)
    );
}

#[test]
fn a_long_record_ttl_is_capped_at_three_hundred_seconds() {
    assert_eq!(
        Fixture::lifetime(EndpointSource::Srv, Some(Duration::from_secs(3600))),
        Duration::from_secs(300)
    );
}

#[test]
fn a_record_ttl_inside_the_range_is_kept() {
    assert_eq!(
        Fixture::lifetime(EndpointSource::Srv, Some(Duration::from_secs(120))),
        Duration::from_secs(120)
    );
}

#[test]
fn a_default_port_lives_thirty_seconds_and_a_typed_port_three_hundred() {
    assert_eq!(
        Fixture::lifetime(EndpointSource::Default, None),
        Duration::from_secs(30)
    );
    assert_eq!(
        Fixture::lifetime(EndpointSource::Explicit, None),
        Duration::from_secs(300)
    );
}

#[tokio::test]
async fn an_explicit_port_does_no_lookup() {
    let responder = DnsResponder::start(DnsScript::Answer {
        port: 30001,
        ttl: 60,
    })
    .await;
    let service = Fixture::service(&responder);

    let endpoint = service
        .resolve("https://brave-otter.play.bedrockvc.stream:8443")
        .await
        .expect("an endpoint");

    assert_eq!(endpoint.port(), 8443);
    assert_eq!(endpoint.source(), EndpointSource::Explicit);
    assert_eq!(responder.queries(), 0);
}

#[tokio::test]
async fn the_sign_in_check_keeps_an_unparsable_address_as_typed() {
    let responder = DnsResponder::start(DnsScript::NxDomain).await;
    let service = Fixture::service(&responder);

    assert_eq!(service.resolve_base_or_identity("https://").await, "https://");
    assert_eq!(
        service.resolve_base_or_identity(PLAY_SERVER).await,
        "https://brave-otter.play.bedrockvc.stream:443"
    );
}

#[tokio::test]
async fn an_unparsable_address_is_an_error() {
    let responder = DnsResponder::start(DnsScript::NxDomain).await;
    let service = Fixture::service(&responder);

    assert!(service.resolve("https://").await.is_err());
}

#[test]
fn a_refresh_that_replaces_an_entry_takes_the_new_entrys_lifetime() {
    let address = ServerAddress::parse(PLAY_SERVER).expect("a parseable address");
    let replacement = CachedEndpoint {
        endpoint: ServerEndpoint::new(address, 443, EndpointSource::Default),
        lifetime: ServerEndpointExpiry::MIN_LIFETIME,
    };

    let lifetime = ServerEndpointExpiry.expire_after_update(
        &PLAY_SERVER.to_string(),
        &replacement,
        Instant::now(),
        Some(ServerEndpointExpiry::MAX_LIFETIME),
    );

    assert_eq!(lifetime, Some(ServerEndpointExpiry::MIN_LIFETIME));
}
