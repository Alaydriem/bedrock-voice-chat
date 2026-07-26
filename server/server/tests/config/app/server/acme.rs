use bvc_server_lib::config::{Acme, AcmeProviderKind};

fn base_cloudflare() -> Acme {
    let mut acme = Acme::default();
    acme.email = "ops@example.com".to_string();
    acme.provider = Some(AcmeProviderKind::Cloudflare);
    acme.api_token = Some("cf-token".to_string());
    acme
}

fn base_acme_dns() -> Acme {
    let mut acme = Acme::default();
    acme.email = "ops@example.com".to_string();
    acme.provider = Some(AcmeProviderKind::AcmeDns);
    acme.server_url = Some("https://acme-dns.example.com".to_string());
    acme.username = Some("user".to_string());
    acme.password = Some("pass".to_string());
    acme.subdomain = Some("d420c923-bbd7-4056-ab64-c3ca54c9b3cf".to_string());
    acme
}

#[test]
fn provider_parses_known_providers() {
    assert_eq!(
        "cloudflare".parse::<AcmeProviderKind>().unwrap(),
        AcmeProviderKind::Cloudflare
    );
    assert_eq!(
        "acme-dns".parse::<AcmeProviderKind>().unwrap(),
        AcmeProviderKind::AcmeDns
    );
}

#[test]
fn provider_rejects_unknown_provider() {
    let err = "route53".parse::<AcmeProviderKind>().unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("route53"), "got: {msg}");
    assert!(msg.contains("cloudflare"), "got: {msg}");
}

#[test]
fn validate_requires_provider() {
    let mut acme = base_cloudflare();
    acme.provider = None;
    let err = acme.validate(&["a.example.com".to_string()]).unwrap_err();
    assert!(format!("{err}").contains("acme.provider"));
}

#[test]
fn effective_domains_default_from_tls_names_skipping_ips() {
    let acme = base_cloudflare();
    let names = vec![
        "s4.example.com".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    assert_eq!(acme.effective_domains(&names).unwrap(), vec!["s4.example.com"]);
}

#[test]
fn effective_domains_explicit_list_wins() {
    let mut acme = base_cloudflare();
    acme.domains = Some(vec!["voice.example.com".to_string()]);
    let names = vec!["other.example.com".to_string()];
    assert_eq!(
        acme.effective_domains(&names).unwrap(),
        vec!["voice.example.com"]
    );
}

#[test]
fn effective_domains_empty_is_an_error() {
    let acme = base_cloudflare();
    let err = acme.effective_domains(&["10.0.0.1".to_string()]).unwrap_err();
    assert!(format!("{err}").to_lowercase().contains("domain"));
}

#[test]
fn validate_requires_cloudflare_token() {
    let mut acme = base_cloudflare();
    acme.api_token = None;
    let err = acme.validate(&["a.example.com".to_string()]).unwrap_err();
    assert!(format!("{err}").contains("api_token"));
}

#[test]
fn validate_requires_acme_dns_fields() {
    let mut acme = base_acme_dns();
    acme.subdomain = None;
    acme.password = None;
    let err = acme.validate(&["a.example.com".to_string()]).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("subdomain"), "got: {msg}");
    assert!(msg.contains("password"), "got: {msg}");
}

#[test]
fn validate_requires_email() {
    let mut acme = base_cloudflare();
    acme.email = String::new();
    let err = acme.validate(&["a.example.com".to_string()]).unwrap_err();
    assert!(format!("{err}").contains("email"));
}

#[test]
fn directory_defaults_to_lets_encrypt_production() {
    assert_eq!(
        Acme::default().directory,
        "https://acme-v02.api.letsencrypt.org/directory"
    );
}
