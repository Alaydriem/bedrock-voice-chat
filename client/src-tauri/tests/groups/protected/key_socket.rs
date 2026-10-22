use std::time::Duration;

use bvc_client_lib::groups::protected::{KeySocket, KeySocketEvent};
use bytes::Bytes;
use common::structs::group_key::GroupKeyFrame;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

async fn next_event(events: &mut tokio::sync::mpsc::Receiver<KeySocketEvent>) -> KeySocketEvent {
    tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("an event within 5s")
        .expect("event queue open")
}

// The server tells a joiner nobody can admit it by closing with a reason. That reason is the only
// thing separating "no member online" from a dropped connection, so it must survive the socket.
#[tokio::test]
async fn key_socket_maps_no_admitter_close_reason() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let welcome = GroupKeyFrame::Welcome { socket: 7 }.to_bytes().unwrap();
        ws.send(Message::Binary(welcome.into())).await.unwrap();
        let Some(Ok(Message::Binary(bytes))) = ws.next().await else {
            panic!("expected the client's frame");
        };
        let received = GroupKeyFrame::from_bytes(&bytes).unwrap();
        ws.send(Message::Close(Some(CloseFrame {
            code: CloseCode::Policy,
            reason: "NoAdmitterOnline".into(),
        })))
        .await
        .unwrap();
        received
    });

    let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{address}"))
        .await
        .unwrap();
    let (outbound, mut events) = KeySocket::spawn(ws);

    assert_eq!(
        next_event(&mut events).await,
        KeySocketEvent::Frame(GroupKeyFrame::Welcome { socket: 7 })
    );
    let sent = GroupKeyFrame::Broadcast(Bytes::from_static(b"hello"));
    outbound.send(sent.clone()).await.unwrap();
    assert_eq!(server.await.unwrap(), sent);
    assert_eq!(
        next_event(&mut events).await,
        KeySocketEvent::Closed(Some("NoAdmitterOnline".to_string()))
    );
}
