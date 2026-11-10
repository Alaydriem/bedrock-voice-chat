use std::net::{SocketAddr, TcpListener};

use bvc_server_lib::demux::PublicListener;
use bvc_server_lib::services::CurrentNonce;
use bvc_server_lib::services::play::PlayPreflightResponder;

// The demultiplexer binds the port with the public listener's socket options. Those must
// find the port free straight after a served request, whatever state the closed connection
// left it in.
#[tokio::test]
async fn it_binds_straight_after_a_request_served_by_the_preflight_responder() {
    let nonce = CurrentNonce::new_shared();
    nonce.set("abc123".to_string());
    let responder = PlayPreflightResponder::start("127.0.0.1:0".parse().unwrap(), nonce)
        .await
        .expect("the responder starts");
    let addr = responder.local_addr();

    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(format!("https://{addr}{}", PlayPreflightResponder::PATH))
        .send()
        .await
        .unwrap();
    responder.stop().await;

    PublicListener::bind(addr).expect("the public listener binds the released port");
}

// The default `server.listen` is `::`. Windows leaves IPV6_V6ONLY on for a plain bind, which
// would refuse an IPv4 peer.
#[tokio::test]
async fn a_wildcard_ipv6_listener_accepts_an_ipv4_peer() {
    if TcpListener::bind("[::1]:0").is_err() {
        return;
    }
    let listener = PublicListener::bind("[::]:0".parse::<SocketAddr>().unwrap())
        .expect("the wildcard listener binds");
    let port = listener.local_addr().unwrap().port();

    let accepted = tokio::spawn(async move { listener.accept().await.map(|_| ()) });
    tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("an IPv4 peer connects");

    accepted.await.unwrap().expect("the listener accepts it");
}
