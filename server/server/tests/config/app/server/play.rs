use bvc_server_lib::config::{PlayTls, Server};

fn with_play() -> Server {
    let mut server = Server::default();
    server.tls.play = Some(PlayTls::default());
    server
}

// The registry's name and certificate cannot be combined with another source: whichever
// the server used would silently discard the other.
#[test]
fn play_and_acme_conflict() {
    let mut config = with_play();
    config.tls.acme = Some(Default::default());

    let conflict = config.tls_source_conflict().expect("a conflict is reported");

    assert!(conflict.contains("tls.play"));
    assert!(conflict.contains("tls.acme"));
    assert!(conflict.contains("mutually exclusive; remove one"));
}

#[test]
fn play_and_manual_paths_conflict() {
    let mut config = with_play();
    config.tls.certificate = "/etc/bvc/cert.pem".to_string();
    config.tls.key = "/etc/bvc/key.pem".to_string();

    let conflict = config.tls_source_conflict().expect("a conflict is reported");

    assert!(conflict.contains("tls.play"));
    assert!(conflict.contains("tls.certificate"));
}

#[test]
fn play_and_enrollment_token_conflict() {
    let mut config = with_play();
    config.enrollment.token = Some("bvcenroll-abc".to_string());

    let conflict = config.tls_source_conflict().expect("a conflict is reported");

    assert!(conflict.contains("tls.play"));
    assert!(conflict.contains("enrollment.token"));
}

#[test]
fn play_alone_is_not_a_conflict() {
    assert!(with_play().tls_source_conflict().is_none());
}

// A blank enrollment token is no token, so it does not make play conflict with it.
#[test]
fn play_beside_a_blank_enrollment_token_is_not_a_conflict() {
    let mut config = with_play();
    config.enrollment.token = Some("  ".to_string());

    assert!(config.tls_source_conflict().is_none());
}

// The registry publishes an address. A hostname would be refused only at the hourly
// check-in, so it is refused at startup.
#[test]
fn a_play_address_that_is_not_an_ip_is_a_problem() {
    let mut config = with_play();
    config.tls.play = Some(PlayTls {
        address: Some("mc.example.com".to_string()),
    });

    let problem = config.tls_play_address_problem().expect("a problem is reported");

    assert!(problem.contains("mc.example.com"));
}

#[test]
fn ip_literals_and_no_address_are_not_problems() {
    let mut config = with_play();
    assert!(config.tls_play_address_problem().is_none());

    for address in ["203.0.113.7", "2001:db8::1"] {
        config.tls.play = Some(PlayTls {
            address: Some(address.to_string()),
        });
        assert!(config.tls_play_address_problem().is_none(), "{address}");
    }
}

#[test]
fn a_blank_play_address_reads_as_absent() {
    let play = PlayTls {
        address: Some("   ".to_string()),
    };

    assert_eq!(play.address(), None);
}
