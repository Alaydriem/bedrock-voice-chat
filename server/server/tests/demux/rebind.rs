//! A restarted server must bind its public port again at once.
//!
//! When a demultiplexer closes a connection first, the server side of that connection holds
//! the port in TIME_WAIT for about a minute. A listener without SO_REUSEADDR cannot bind the
//! port in that time, so a quick restart (a panel restart with players connected) left the
//! server with no public listener.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use bvc_server_lib::demux::{AlpnDemux, ApiBind, DemuxError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

/// A backend that accepts and holds connections, so the demultiplexer's readiness check passes.
async fn backend() -> SocketAddr {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("bind backend");
    let addr = listener.local_addr().expect("backend addr");
    tokio::spawn(async move { while listener.accept().await.is_ok() {} });
    addr
}

fn start(port: u16, api: SocketAddr) -> JoinHandle<Result<(), DemuxError>> {
    let demux = AlpnDemux::new(
        SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        ApiBind::new(api.port()),
        None,
    );
    tokio::spawn(async move { demux.start().await })
}

async fn connect(port: u16) -> TcpStream {
    for _ in 0..100 {
        if let Ok(stream) = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).await
        {
            return stream;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the demultiplexer never listened on {port}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_restart_binds_the_port_a_closed_connection_still_holds() {
    let api = backend().await;
    let port = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .await
        .expect("reserve a port")
        .local_addr()
        .expect("reserved addr")
        .port();

    let first = start(port, api);
    let mut client = connect(port).await;

    // Bytes that are not a ClientHello: the demultiplexer drops the connection, so its side
    // closes first and holds the port in TIME_WAIT. The client waits for that close before it
    // closes its own side.
    client
        .write_all(b"GET / HTTP/1.1\r\n\r\n")
        .await
        .expect("write to the demultiplexer");
    let mut rest = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), client.read_to_end(&mut rest)).await;
    drop(client);

    first.abort();
    let _ = first.await;

    let second = start(port, api);
    // `start` only returns on an error; one still running after this has bound the port.
    match tokio::time::timeout(Duration::from_secs(3), second).await {
        Err(_) => {}
        Ok(Ok(Err(e))) => panic!("the restarted demultiplexer could not bind: {e}"),
        Ok(other) => panic!("the restarted demultiplexer stopped: {other:?}"),
    }
}
