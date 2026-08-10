//! Which backend a connection reaches, decided from the ClientHello alone.
//!
//! The demultiplexer does not terminate TLS, so these tests never complete a handshake.
//! They send a real ClientHello, then assert which backend received the bytes — which is
//! the entire routing contract.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use bvc_server_lib::demux::AlpnDemux;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// A stand-in backend that reports the first bytes it is handed.
async fn spy_backend() -> (SocketAddr, mpsc::Receiver<usize>) {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("bind spy backend");
    let addr = listener.local_addr().expect("spy backend addr");
    let (tx, rx) = mpsc::channel(4);

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buffer = [0u8; 1024];
                if let Ok(read) = stream.read(&mut buffer).await
                    && read > 0
                {
                    let _ = tx.send(read).await;
                }
            });
        }
    });

    (addr, rx)
}

/// Sends a genuine ClientHello offering `alpn`, then walks away.
///
/// A raw byte blob would not exercise the rustls parse the demultiplexer performs, so the
/// hello is produced by rustls itself. The handshake never completes — no backend here
/// speaks TLS — and that is fine: routing is decided before any reply.
async fn offer_alpn(port: u16, alpn: &[&[u8]]) {
    // rustls has no ambient provider in a test process, and `builder()` panics rather
    // than erroring when one is missing.
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("default protocol versions")
    .with_root_certificates(rustls::RootCertStore::empty())
    .with_no_client_auth();
    config.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();

    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let stream = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
        .await
        .expect("connect to the demultiplexer");
    let name = rustls::pki_types::ServerName::try_from("bvc.test").expect("server name");

    // The handshake cannot finish, so it is bounded rather than awaited to completion.
    let _ = tokio::time::timeout(Duration::from_millis(500), connector.connect(name, stream)).await;
}

async fn demux_on(api: SocketAddr, websocket: Option<SocketAddr>) -> u16 {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("reserve a demultiplexer port");
    let port = listener.local_addr().expect("demux addr").port();
    drop(listener);

    let demux = AlpnDemux::new(SocketAddr::from((Ipv4Addr::LOCALHOST, port)), api, websocket);
    tokio::spawn(async move {
        let _ = demux.start().await;
    });

    // `start` waits for both backends before it binds, so the port answering is the signal
    // that routing is live.
    for _ in 0..100 {
        if TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
            .await
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    port
}

async fn received(rx: &mut mpsc::Receiver<usize>) -> bool {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .ok()
        .flatten()
        .is_some()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_voice_protocol_reaches_the_websocket_listener() {
    let (api, mut api_rx) = spy_backend().await;
    let (websocket, mut websocket_rx) = spy_backend().await;
    let port = demux_on(api, Some(websocket)).await;

    offer_alpn(port, &[AlpnDemux::WEBSOCKET_ALPN]).await;

    assert!(
        received(&mut websocket_rx).await,
        "a client offering bvc-ws/1 must reach the WebSocket listener"
    );
    assert!(
        !received(&mut api_rx).await,
        "the API must not see the voice transport"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_protocols_reach_the_api() {
    let (api, mut api_rx) = spy_backend().await;
    let (websocket, mut websocket_rx) = spy_backend().await;
    let port = demux_on(api, Some(websocket)).await;

    offer_alpn(port, &[b"h2", b"http/1.1"]).await;

    assert!(
        received(&mut api_rx).await,
        "an HTTP client must reach the API"
    );
    assert!(
        !received(&mut websocket_rx).await,
        "the WebSocket listener must not see API traffic"
    );
}

/// The load-bearing default. A browser cannot set ALPN, and the position feed is a browser
/// socket, so a hello offering nothing must reach the API rather than being refused.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hello_with_no_protocol_reaches_the_api() {
    let (api, mut api_rx) = spy_backend().await;
    let (websocket, mut websocket_rx) = spy_backend().await;
    let port = demux_on(api, Some(websocket)).await;

    offer_alpn(port, &[]).await;

    assert!(
        received(&mut api_rx).await,
        "a client offering no protocol must reach the API"
    );
    assert!(
        !received(&mut websocket_rx).await,
        "the WebSocket listener must not see a hello that asked for nothing"
    );
}

/// With no WebSocket listener configured, a client that asks for one is refused outright
/// rather than relayed into the API, which would answer a WebSocket upgrade with an HTTP
/// error and leave the client guessing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_voice_protocol_is_refused_when_no_listener_exists() {
    let (api, mut api_rx) = spy_backend().await;
    let port = demux_on(api, None).await;

    offer_alpn(port, &[AlpnDemux::WEBSOCKET_ALPN]).await;

    assert!(
        !received(&mut api_rx).await,
        "a voice client must not be relayed into the API when no WebSocket listener exists"
    );
}
