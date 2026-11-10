use std::str::FromStr;

use bvc_relay_service::config::PlayConfig;
use bvc_relay_service::play::PlayCaResolver;
use reqwest::dns::{Name, Resolve};

// The CA's fixed name goes to the configured endpoint, looked up at the time of the
// connection rather than once at startup.
#[tokio::test]
async fn the_ca_name_resolves_to_the_configured_endpoint() {
    let resolver = PlayCaResolver::new("localhost:9000".parse().expect("an endpoint"));

    let addresses: Vec<_> = resolver
        .resolve(Name::from_str(PlayConfig::CA_HOST).expect("a name"))
        .await
        .expect("localhost resolves")
        .collect();

    assert!(!addresses.is_empty());
    assert!(addresses.iter().all(|address| address.ip().is_loopback()));
}

// The CA client must never send another name to public DNS, where an attacker who controls
// the answer could stand in for the CA.
#[tokio::test]
async fn any_other_name_is_refused() {
    let resolver = PlayCaResolver::new("localhost:9000".parse().expect("an endpoint"));

    for name in ["example.com", "registry.bedrockvoicechat.com", "evil.play.bedrockvc.stream"] {
        let result = resolver.resolve(Name::from_str(name).expect("a name")).await;

        assert!(result.is_err(), "{name}");
    }
}

// A name that does not resolve fails the lookup instead of falling back to anything else.
#[tokio::test]
async fn an_endpoint_that_does_not_resolve_is_an_error() {
    let resolver = PlayCaResolver::new("no-such-host.invalid:9000".parse().expect("an endpoint"));

    let result = resolver
        .resolve(Name::from_str(PlayConfig::CA_HOST).expect("a name"))
        .await;

    assert!(result.is_err());
}
