use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::routing::get;
use axum_server::tls_rustls::RustlsConfig;
use bvc_relay_service::validation::{LiveAddressProbe, ProbeOutcome};
use time::Duration as Validity;
use tokio::net::TcpListener;

use crate::harness::CertificateFixture;

const NAME: &str = "preflight.test";
const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

struct ProbeFixture;

impl ProbeFixture {
    fn probe() -> LiveAddressProbe {
        LiveAddressProbe::reachability(Duration::from_secs(3), Duration::from_secs(3))
    }

    // A TLS responder on an ephemeral loopback port that answers every nonce request with
    // `body`.
    async fn responder(body: &'static str) -> SocketAddr {
        Self::serve(Router::new().route(
            "/health/enrollment-nonce",
            get(move || async move { body }),
        ))
        .await
    }

    async fn serve(app: Router) -> SocketAddr {
        let fixture = CertificateFixture::issue(NAME, Validity::days(1));
        let config = RustlsConfig::from_pem(
            fixture.material.chain_pem.into_bytes(),
            fixture.material.key_pem.into_bytes(),
        )
        .await
        .expect("a TLS config");
        let listener = std::net::TcpListener::bind((LOOPBACK, 0)).expect("a loopback listener");
        let addr = listener.local_addr().expect("a bound address");
        tokio::spawn(async move {
            axum_server::from_tcp_rustls(listener, config)
                .serve(app.into_make_service())
                .await
        });
        addr
    }
}

#[tokio::test]
async fn a_redirect_is_wrong_service_and_is_not_followed() {
    let target = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let location = format!("https://{}/", target.local_addr().unwrap());
    let addr = ProbeFixture::serve(Router::new().route(
        "/health/enrollment-nonce",
        get(move || async move { (StatusCode::FOUND, [(header::LOCATION, location)]) }),
    ))
    .await;

    let outcome = ProbeFixture::probe().probe(NAME, LOOPBACK, addr.port(), "expected").await;

    assert_eq!(outcome, ProbeOutcome::WrongService);
    let connected = tokio::time::timeout(Duration::from_millis(500), target.accept()).await;
    assert!(connected.is_err(), "the redirect target was dialled");
}

#[tokio::test]
async fn a_body_larger_than_a_nonce_is_wrong_service_even_around_the_right_nonce() {
    let body = format!("expected{}", " ".repeat(4096));
    let addr = ProbeFixture::serve(Router::new().route(
        "/health/enrollment-nonce",
        get(move || {
            let body = body.clone();
            async move { body }
        }),
    ))
    .await;

    assert_eq!(
        ProbeFixture::probe().probe(NAME, LOOPBACK, addr.port(), "expected").await,
        ProbeOutcome::WrongService
    );
}

#[tokio::test]
async fn a_chunked_body_larger_than_a_nonce_is_wrong_service() {
    let addr = ProbeFixture::serve(Router::new().route(
        "/health/enrollment-nonce",
        get(|| async {
            let chunks = (0..64).map(|_| Ok::<_, std::convert::Infallible>(vec![b' '; 64]));
            let mut padded = vec![Ok(b"expected".to_vec())];
            padded.extend(chunks);
            Body::from_stream(futures_util::stream::iter(padded))
        }),
    ))
    .await;

    assert_eq!(
        ProbeFixture::probe().probe(NAME, LOOPBACK, addr.port(), "expected").await,
        ProbeOutcome::WrongService
    );
}

#[tokio::test]
async fn a_closed_port_is_refused() {
    let listener = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    assert_eq!(ProbeFixture::probe().probe(NAME, LOOPBACK, port, "n").await, ProbeOutcome::Refused);
}

#[tokio::test]
async fn a_listener_that_never_answers_times_out() {
    let listener = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let held = tokio::spawn(async move {
        let mut sockets = Vec::new();
        while let Ok((socket, _)) = listener.accept().await {
            sockets.push(socket);
        }
    });

    let probe = LiveAddressProbe::reachability(Duration::from_secs(1), Duration::from_secs(1));
    assert_eq!(probe.probe(NAME, LOOPBACK, port, "n").await, ProbeOutcome::TimedOut);
    held.abort();
}

#[tokio::test]
async fn the_wrong_nonce_is_wrong_service() {
    let addr = ProbeFixture::responder("other").await;

    assert_eq!(
        ProbeFixture::probe().probe(NAME, LOOPBACK, addr.port(), "expected").await,
        ProbeOutcome::WrongService
    );
}

#[tokio::test]
async fn the_right_nonce_is_answered() {
    let addr = ProbeFixture::responder("expected").await;

    assert_eq!(
        ProbeFixture::probe().probe(NAME, LOOPBACK, addr.port(), "expected").await,
        ProbeOutcome::Answered
    );
}
