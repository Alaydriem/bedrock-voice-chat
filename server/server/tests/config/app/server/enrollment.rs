use bvc_server_lib::config::Server;

fn with_enrollment_token() -> Server {
    let mut server = Server::default();
    server.enrollment.token = Some("bvcenroll-abc".to_string());
    server
}

// Exactly one source of certificate material. Two set is a configuration the server
// cannot resolve, and guessing which the operator meant is worse than refusing.
#[test]
fn an_enrollment_token_beside_manual_certificate_paths_is_a_conflict() {
    let mut config = with_enrollment_token();
    config.tls.certificate = "/etc/bvc/cert.pem".to_string();
    config.tls.key = "/etc/bvc/key.pem".to_string();

    let conflict = config.tls_source_conflict().expect("a conflict is reported");

    assert!(conflict.contains("enrollment.token"));
    assert!(conflict.contains("tls.certificate"));
}

#[test]
fn an_enrollment_token_beside_an_acme_block_is_a_conflict() {
    let mut config = with_enrollment_token();
    config.tls.acme = Some(Default::default());

    let conflict = config.tls_source_conflict().expect("a conflict is reported");

    assert!(conflict.contains("enrollment.token"));
    assert!(conflict.contains("tls.acme"));
}

#[test]
fn manual_paths_beside_an_acme_block_are_a_conflict() {
    let mut config = Server::default();
    config.tls.certificate = "/etc/bvc/cert.pem".to_string();
    config.tls.key = "/etc/bvc/key.pem".to_string();
    config.tls.acme = Some(Default::default());

    let conflict = config.tls_source_conflict().expect("a conflict is reported");

    assert!(conflict.contains("tls.acme"));
    assert!(conflict.contains("tls.certificate"));
}

#[test]
fn exactly_one_source_is_not_a_conflict() {
    assert!(with_enrollment_token().tls_source_conflict().is_none());
}

// A blank token is no token. An operator who cleared the value rather than deleting
// the line has configured nothing, and reading it as a third source would refuse a
// server that is otherwise correctly configured.
#[test]
fn a_blank_enrollment_token_is_not_a_source() {
    let mut config = Server::default();
    config.enrollment.token = Some("   ".to_string());
    config.tls.certificate = "/etc/bvc/cert.pem".to_string();
    config.tls.key = "/etc/bvc/key.pem".to_string();

    assert!(config.tls_source_conflict().is_none());
}

// No source at all is not a conflict here. It is refused later, by
// `get_rocket_config`, with an error naming the missing certificate file — which is
// the more useful message for an operator who has not configured TLS yet.
#[test]
fn no_source_at_all_is_not_a_conflict() {
    assert!(Server::default().tls_source_conflict().is_none());
}

// Blank values are absent values throughout the block. An operator who cleared a line
// rather than deleting it has configured nothing, and a blank address would otherwise
// ask the registry to publish an empty A record.
#[test]
fn blank_enrollment_values_read_as_absent() {
    let mut config = Server::default();
    config.enrollment.token = Some("  ".to_string());
    config.enrollment.address = Some("".to_string());

    assert_eq!(config.enrollment.token(), None);
    assert_eq!(config.enrollment.address(), None);
}

// The address is optional and independent of the token. A server enrolling without
// one publishes no A record at all, which is right: BVC clients reach it regardless,
// and only an off-host Bedrock addon needs the name to resolve.
#[test]
fn an_address_is_optional_alongside_a_token() {
    let mut config = Server::default();
    config.enrollment.token = Some("bvcenroll-abc".to_string());

    assert!(config.enrollment.token().is_some());
    assert_eq!(config.enrollment.address(), None);
    assert!(config.tls_source_conflict().is_none());
}
