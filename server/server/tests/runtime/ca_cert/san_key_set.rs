use bvc_server_lib::runtime::ca_cert::SanKeySet;
use rcgen::{CertificateParams, KeyPair};

fn s(v: &str) -> String {
    v.to_string()
}

fn build_test_cert_pem(san_strings: &[String]) -> String {
    let kp = KeyPair::generate().unwrap();
    let params = CertificateParams::new(san_strings.to_vec()).unwrap();
    let cert = params.self_signed(&kp).unwrap();
    cert.pem()
}

#[test]
fn a_dns_name_normalizes_to_a_lowercased_dns_key() {
    let set = SanKeySet::from_strings(&[s("Example.COM")]).unwrap();
    assert_eq!(set.sorted(), vec![s("DNS:example.com")]);
}

#[test]
fn an_ipv4_address_normalizes_to_an_ip_key() {
    let set = SanKeySet::from_strings(&[s("127.0.0.1")]).unwrap();
    assert_eq!(set.sorted(), vec![s("IP:127.0.0.1")]);
}

#[test]
fn an_ipv6_address_normalizes_to_its_canonical_display_form() {
    let set = SanKeySet::from_strings(&[s("::1")]).unwrap();
    assert_eq!(set.sorted(), vec![s("IP:::1")]);
}

#[test]
fn a_mixed_set_keeps_dns_and_ip_entries_distinct() {
    let set = SanKeySet::from_strings(&[s("localhost"), s("127.0.0.1")]).unwrap();
    assert!(set.contains("DNS:localhost"));
    assert!(set.contains("IP:127.0.0.1"));
    assert_eq!(set.len(), 2);
}

#[test]
fn the_key_set_is_order_and_duplicate_insensitive() {
    let a = SanKeySet::from_strings(&[s("a.example"), s("b.example")]).unwrap();
    let b =
        SanKeySet::from_strings(&[s("b.example"), s("a.example"), s("a.example")]).unwrap();
    assert_eq!(a, b);
}

#[test]
fn dns_names_compare_case_insensitively() {
    let a = SanKeySet::from_strings(&[s("Example.COM")]).unwrap();
    let b = SanKeySet::from_strings(&[s("example.com")]).unwrap();
    assert_eq!(a, b);
}

#[test]
fn a_certificate_pem_yields_the_sans_it_embeds() {
    let pem = build_test_cert_pem(&[s("localhost"), s("127.0.0.1")]);
    let set = SanKeySet::from_certificate_pem(&pem).unwrap();
    assert!(set.contains("DNS:localhost"));
    assert!(set.contains("IP:127.0.0.1"));
}

#[test]
fn a_certificate_with_no_san_extension_yields_an_empty_set() {
    let kp = KeyPair::generate().unwrap();
    let params = CertificateParams::new(Vec::<String>::new()).unwrap();
    let cert = params.self_signed(&kp).unwrap();
    let set = SanKeySet::from_certificate_pem(&cert.pem()).unwrap();
    assert!(set.is_empty());
}

#[test]
fn a_garbage_pem_is_an_error_rather_than_an_empty_set() {
    let err = SanKeySet::from_certificate_pem("not a real PEM").unwrap_err();
    assert!(format!("{err}").contains("PEM"));
}

#[test]
fn a_certificate_round_trips_against_the_strings_that_produced_it() {
    let names = [s("localhost"), s("127.0.0.1")];
    let from_config = SanKeySet::from_strings(&names).unwrap();
    let from_cert = SanKeySet::from_certificate_pem(&build_test_cert_pem(&names)).unwrap();
    assert_eq!(
        from_config, from_cert,
        "a cert generated from a SAN set must parse back to the same set, \
         or `ensure` re-signs on every start"
    );
}
