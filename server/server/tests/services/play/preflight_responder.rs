use std::net::{SocketAddr, TcpListener};

use bvc_server_lib::services::CurrentNonce;
use bvc_server_lib::services::play::PlayPreflightResponder;

struct Http;

impl Http {
    fn loopback() -> SocketAddr {
        "127.0.0.1:0".parse().unwrap()
    }

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap()
    }
}

#[tokio::test]
async fn the_nonce_is_served_once_set() {
    let nonce = CurrentNonce::new_shared();
    let responder = PlayPreflightResponder::start(Http::loopback(), nonce.clone())
        .await
        .expect("the responder starts");
    let url = format!(
        "https://{}{}",
        responder.local_addr(),
        PlayPreflightResponder::PATH
    );

    let before = Http::client().get(&url).send().await.unwrap();
    assert_eq!(before.status().as_u16(), 404);

    nonce.set("abc123".to_string());
    let after = Http::client().get(&url).send().await.unwrap();
    assert_eq!(after.status().as_u16(), 200);
    assert_eq!(after.text().await.unwrap(), "abc123");

    responder.stop().await;
}

#[tokio::test]
async fn other_paths_are_not_found() {
    let nonce = CurrentNonce::new_shared();
    nonce.set("abc123".to_string());
    let responder = PlayPreflightResponder::start(Http::loopback(), nonce)
        .await
        .expect("the responder starts");

    let response = Http::client()
        .get(format!("https://{}/api/config", responder.local_addr()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 404);
    responder.stop().await;
}

// The demultiplexer binds this port next. The responder must have released it, with the
// probe's connection already served.
#[tokio::test]
async fn the_port_is_free_after_stop() {
    let nonce = CurrentNonce::new_shared();
    nonce.set("abc123".to_string());
    let responder = PlayPreflightResponder::start(Http::loopback(), nonce)
        .await
        .expect("the responder starts");
    let addr = responder.local_addr();

    Http::client()
        .get(format!("https://{addr}{}", PlayPreflightResponder::PATH))
        .send()
        .await
        .unwrap();
    responder.stop().await;

    TcpListener::bind(addr).expect("the port is free once the responder stops");
}

// The default `server.listen` is `::`. Windows leaves IPV6_V6ONLY on for a plain bind, which
// would refuse the IPv4 address the registry probes.
#[tokio::test]
async fn a_wildcard_ipv6_listen_address_also_serves_ipv4() {
    if TcpListener::bind("[::1]:0").is_err() {
        return;
    }
    let nonce = CurrentNonce::new_shared();
    nonce.set("abc123".to_string());
    let responder = PlayPreflightResponder::start("[::]:0".parse().unwrap(), nonce)
        .await
        .expect("the responder starts");
    let port = responder.local_addr().port();

    let response = Http::client()
        .get(format!(
            "https://127.0.0.1:{port}{}",
            PlayPreflightResponder::PATH
        ))
        .send()
        .await
        .expect("an IPv4 client reaches the responder");

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.text().await.unwrap(), "abc123");
    responder.stop().await;
}

// A connection that sends nothing must not keep the port after stop. The request timeout
// is five seconds, so a stop that waited for it would be seen here.
#[tokio::test]
async fn stop_closes_an_idle_connection_and_frees_the_port() {
    let nonce = CurrentNonce::new_shared();
    let responder = PlayPreflightResponder::start(Http::loopback(), nonce)
        .await
        .expect("the responder starts");
    let addr = responder.local_addr();
    let _idle = tokio::net::TcpStream::connect(addr)
        .await
        .expect("the idle connection opens");

    tokio::time::timeout(std::time::Duration::from_secs(2), responder.stop())
        .await
        .expect("stop does not wait for the idle connection");

    TcpListener::bind(addr).expect("the port is free once the responder stops");
}
