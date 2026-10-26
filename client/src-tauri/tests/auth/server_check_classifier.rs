use bvc_client_lib::ServerCheckClassifier;
use common::structs::reachability::ServerCheckFailure;
use rustls::pki_types::ServerName;

// tokio-rustls hands a rustls failure to hyper inside an io::Error, which is the shape a real
// handshake failure arrives in.
fn as_io(error: rustls::Error) -> std::io::Error {
    std::io::Error::other(error)
}

#[tokio::test]
async fn a_closed_local_port_is_refused() {
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
        listener.local_addr().expect("local addr").port()
    };

    let error = reqwest::Client::new()
        .get(format!("https://127.0.0.1:{port}/api/config"))
        .send()
        .await
        .expect_err("nothing listens on a port just released");

    assert_eq!(ServerCheckClassifier::classify(&error), ServerCheckFailure::Refused);
}

// The most useful thing this screen can say to someone who typed their game host: which name
// the certificate there really belongs to.
#[test]
fn a_name_mismatch_carries_the_presented_names() {
    let error = as_io(rustls::Error::InvalidCertificate(
        rustls::CertificateError::NotValidForNameContext {
            expected: ServerName::try_from("restcraft.net").unwrap().to_owned(),
            presented: vec!["DnsName(\"panel.restcraft.net\")".to_string()],
        },
    ));

    assert_eq!(
        ServerCheckClassifier::classify(&error),
        ServerCheckFailure::WrongCertificateName {
            presented: vec!["panel.restcraft.net".to_string()],
        }
    );
}

#[test]
fn another_tls_error_is_tls_rejected() {
    let error = as_io(rustls::Error::AlertReceived(
        rustls::AlertDescription::InternalError,
    ));

    assert_eq!(ServerCheckClassifier::classify(&error), ServerCheckFailure::TlsRejected);
}

// Outlook answering 401 for an email typed as an address: something answered, and it is not BVC.
#[test]
fn a_client_error_status_is_not_bvc() {
    assert_eq!(
        ServerCheckClassifier::for_status(401),
        ServerCheckFailure::NotBvc { status: Some(401) }
    );
}

// A BVC server restarting behind nginx or Cloudflare answers 502/503/504; a rate limit is 429.
// The address is right, so the player must not be told it is the wrong kind of server.
#[test]
fn a_server_error_or_rate_limit_is_a_server_error() {
    for status in [500, 502, 503, 504, 429] {
        assert_eq!(
            ServerCheckClassifier::for_status(status),
            ServerCheckFailure::ServerError { status },
            "status {status}"
        );
    }
}
