use bvc_relay_service::config::PlayCaConfig;
use bvc_relay_service::play::OneTimeToken;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use rcgen::KeyPair;

use crate::harness::PlayFixture;

fn config_with(key: &KeyPair) -> PlayCaConfig {
    let mut config = PlayFixture::config().ca;
    config.provisioner_key = key.serialize_pem();
    config.provisioner_kid = "kid-1".to_string();
    config
}

fn decode(token: &str, key: &KeyPair, audience: &str) -> serde_json::Value {
    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_audience(&[audience]);
    validation.leeway = 0;
    validation.validate_nbf = true;

    jsonwebtoken::decode::<serde_json::Value>(
        token,
        &DecodingKey::from_ec_pem(key.public_key_pem().as_bytes()).expect("a public key"),
        &validation,
    )
    .expect("the token verifies against the provisioner's public key")
    .claims
}

#[test]
fn the_token_carries_one_san_and_expires_in_five_minutes() {
    let key = KeyPair::generate().expect("a key generates");
    let config = config_with(&key);
    let fqdn = "brave-otter-mesa.play.bedrockvc.stream";

    let token = OneTimeToken::mint(&config, fqdn).expect("mints");
    let claims = decode(&token, &key, "https://ca.play.bedrockvc.stream:9000/1.0/sign");

    assert_eq!(claims["sans"], serde_json::json!([fqdn]));
    assert_eq!(claims["sub"], fqdn);
    assert_eq!(claims["aud"], "https://ca.play.bedrockvc.stream:9000/1.0/sign");
    assert_eq!(claims["iss"], "registry");
    assert_eq!(
        claims["exp"].as_u64().expect("exp") - claims["iat"].as_u64().expect("iat"),
        300
    );
    assert_eq!(
        claims["iat"].as_u64().expect("iat") - claims["nbf"].as_u64().expect("nbf"),
        30
    );
}

#[test]
fn the_header_names_the_provisioner_key() {
    let key = KeyPair::generate().expect("a key generates");
    let token = OneTimeToken::mint(&config_with(&key), "a.play.bedrockvc.stream").expect("mints");

    let header = jsonwebtoken::decode_header(&token).expect("a header");

    assert_eq!(header.alg, Algorithm::ES256);
    assert_eq!(header.kid.as_deref(), Some("kid-1"));
}

// step-ca spends a token even when it refuses the request, so a retry that reused one
// would fail on the replay and hide the first error.
#[test]
fn two_tokens_for_one_name_differ() {
    let key = KeyPair::generate().expect("a key generates");
    let config = config_with(&key);

    let first = OneTimeToken::mint(&config, "a.play.bedrockvc.stream").expect("mints");
    let second = OneTimeToken::mint(&config, "a.play.bedrockvc.stream").expect("mints");

    assert_ne!(first, second);
}
