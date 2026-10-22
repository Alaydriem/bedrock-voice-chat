//! `GET /api/websocket/group/<id>`
//!
//! Contract:
//! - only a member of a protected channel may open its key socket
//! - the first frame is `Welcome` naming the socket's own id
//! - commits are sequenced and delivered to every socket
//! - when the last socket closes the group has ended, and its channel is deleted

use std::time::Duration;

use bytes::Bytes;
use common::Game;
use common::request::channel::ProtectedChannelCreateRequest;
use common::structs::channel::Channel;
use common::structs::group_key::{GroupKeyFrame, GroupKeyRecipient, SocketId};
use rocket::futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::harness::http_client::MtlsClient;
use crate::harness::mtls_websocket::{MtlsSocket, MtlsWebSocket};
use crate::harness::{HttpAssert, TestServer};

struct Player {
    cert: String,
    key: String,
}

async fn player(env: &TestServer, name: &str) -> Player {
    let (cert, key) = env.issue_player(name, &Game::Minecraft).await.unwrap();
    Player { cert, key }
}

async fn create(env: &TestServer, who: &Player, protected: bool) -> String {
    let client = MtlsClient::with_identity(&env.ca_pem, &who.cert, &who.key).unwrap();
    let response = if protected {
        client
            .post(format!("{}/api/channel/protected", env.base_url))
            .json(&ProtectedChannelCreateRequest {
                name: "vault".to_string(),
            })
            .send()
            .await
            .unwrap()
    } else {
        client
            .post(format!("{}/api/channel", env.base_url))
            .json(&"open".to_string())
            .send()
            .await
            .unwrap()
    };
    HttpAssert::status(response.status().as_u16(), 200);
    response.text().await.unwrap().trim_matches('"').to_string()
}

async fn join(env: &TestServer, who: &Player, id: &str, protected: bool) {
    let client = MtlsClient::with_identity(&env.ca_pem, &who.cert, &who.key).unwrap();
    let response = if protected {
        client
            .put(format!("{}/api/channel/{}/protected", env.base_url, id))
            .send()
            .await
            .unwrap()
    } else {
        client
            .put(format!("{}/api/channel/{}", env.base_url, id))
            .json(&common::structs::channel::ChannelEvent::new(
                common::structs::channel::ChannelEvents::Join,
            ))
            .send()
            .await
            .unwrap()
    };
    HttpAssert::status(response.status().as_u16(), 200);
}

async fn listed(env: &TestServer, who: &Player, id: &str) -> bool {
    let response = MtlsClient::with_identity(&env.ca_pem, &who.cert, &who.key)
        .unwrap()
        .get(format!("{}/api/channel?id={}", env.base_url, id))
        .send()
        .await
        .unwrap();
    if response.status().as_u16() != 200 {
        return false;
    }
    !response.json::<Vec<Channel>>().await.unwrap().is_empty()
}

async fn socket(env: &TestServer, who: &Player, id: &str) -> Result<MtlsSocket, u16> {
    MtlsWebSocket::connect(
        &env.base_url,
        &env.ca_pem,
        &who.cert,
        &who.key,
        &format!("/api/websocket/group/{id}"),
    )
    .await
}

async fn next_frame(socket: &mut MtlsSocket) -> GroupKeyFrame {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .expect("a frame within 5s")
            .expect("socket open")
            .expect("frame");
        if let Message::Binary(bytes) = message {
            return GroupKeyFrame::from_bytes(&bytes).expect("a group key frame");
        }
    }
}

async fn send(socket: &mut MtlsSocket, frame: GroupKeyFrame) {
    socket
        .send(Message::Binary(frame.to_bytes().unwrap().into()))
        .await
        .unwrap();
}

async fn welcomed(socket: &mut MtlsSocket) -> SocketId {
    match next_frame(socket).await {
        GroupKeyFrame::Welcome { socket } => socket,
        other => panic!("expected Welcome, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn group_socket_refuses_a_non_member() {
    let env = TestServer::start().await.unwrap();
    let creator = player(&env, "Creator").await;
    let outsider = player(&env, "Outsider").await;
    let id = create(&env, &creator, true).await;

    let status = socket(&env, &outsider, &id).await.err();

    assert_eq!(status, Some(403));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn group_socket_refuses_an_unprotected_channel() {
    let env = TestServer::start().await.unwrap();
    let creator = player(&env, "Creator").await;
    let id = create(&env, &creator, false).await;
    join(&env, &creator, &id, false).await;

    let status = socket(&env, &creator, &id).await.err();

    assert_eq!(status, Some(409));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn group_socket_refuses_a_missing_channel() {
    let env = TestServer::start().await.unwrap();
    let creator = player(&env, "Creator").await;

    let status = socket(&env, &creator, "ZZZZZZZZ").await.err();

    assert_eq!(status, Some(404));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn group_socket_sends_welcome_with_socket_id() {
    let env = TestServer::start().await.unwrap();
    let creator = player(&env, "Creator").await;
    let id = create(&env, &creator, true).await;
    join(&env, &creator, &id, true).await;

    let mut ws = socket(&env, &creator, &id).await.expect("socket opens");

    assert!(welcomed(&mut ws).await > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn commit_round_trips_between_two_members() {
    let env = TestServer::start().await.unwrap();
    let alice = player(&env, "Alice").await;
    let bob = player(&env, "Bob").await;
    let id = create(&env, &alice, true).await;
    join(&env, &alice, &id, true).await;
    join(&env, &bob, &id, true).await;
    let mut a = socket(&env, &alice, &id).await.unwrap();
    let a_id = welcomed(&mut a).await;
    let mut b = socket(&env, &bob, &id).await.unwrap();
    let b_id = welcomed(&mut b).await;

    // A member announces its binding on joining; only announced sockets are admitters.
    send(&mut a, GroupKeyFrame::Broadcast(Bytes::from_static(b"bind"))).await;
    assert_eq!(
        next_frame(&mut b).await,
        GroupKeyFrame::Delivered {
            from: a_id,
            body: Bytes::from_static(b"bind"),
        }
    );

    send(
        &mut b,
        GroupKeyFrame::Direct {
            to: GroupKeyRecipient::Any,
            body: Bytes::from_static(b"pake-init"),
        },
    )
    .await;
    assert_eq!(
        next_frame(&mut a).await,
        GroupKeyFrame::Delivered {
            from: b_id,
            body: Bytes::from_static(b"pake-init"),
        }
    );

    let commit = GroupKeyFrame::Commit {
        epoch: 0,
        body: Bytes::from_static(b"add-bob"),
    };
    send(&mut a, commit.clone()).await;
    assert_eq!(next_frame(&mut a).await, commit);
    assert_eq!(next_frame(&mut b).await, commit);
    assert_ne!(a_id, b_id);
}

// A joiner must learn at once that nobody can admit it, not wait on a socket that will never
// answer — and the group it was trying to enter is gone with its last member.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn joiner_after_only_member_leaves_gets_no_admitter_and_channel_is_gone() {
    let env = TestServer::start().await.unwrap();
    let alice = player(&env, "Alice").await;
    let bob = player(&env, "Bob").await;
    let id = create(&env, &alice, true).await;
    join(&env, &alice, &id, true).await;
    join(&env, &bob, &id, true).await;
    let mut a = socket(&env, &alice, &id).await.unwrap();
    let a_id = welcomed(&mut a).await;
    let mut b = socket(&env, &bob, &id).await.unwrap();
    welcomed(&mut b).await;

    a.close(None).await.unwrap();
    assert_eq!(next_frame(&mut b).await, GroupKeyFrame::Gone { member: a_id });
    send(
        &mut b,
        GroupKeyFrame::Direct {
            to: GroupKeyRecipient::Any,
            body: Bytes::from_static(b"pake-init"),
        },
    )
    .await;

    let close = loop {
        match tokio::time::timeout(Duration::from_secs(5), b.next())
            .await
            .expect("the server answers within 5s")
        {
            Some(Ok(Message::Close(frame))) => break frame,
            Some(Ok(_)) => continue,
            other => panic!("expected a close frame, got {other:?}"),
        }
    };
    assert_eq!(close.expect("a close frame with a reason").reason.as_str(), "NoAdmitterOnline");

    let mut deleted = false;
    for _ in 0..50 {
        if !listed(&env, &bob, &id).await {
            deleted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(deleted, "the channel outlived its last member");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn last_socket_close_deletes_the_channel() {
    let env = TestServer::start().await.unwrap();
    let alice = player(&env, "Alice").await;
    let bob = player(&env, "Bob").await;
    let id = create(&env, &alice, true).await;
    join(&env, &alice, &id, true).await;
    join(&env, &bob, &id, true).await;
    let mut a = socket(&env, &alice, &id).await.unwrap();
    let a_id = welcomed(&mut a).await;
    let mut b = socket(&env, &bob, &id).await.unwrap();
    welcomed(&mut b).await;

    a.close(None).await.unwrap();
    assert_eq!(next_frame(&mut b).await, GroupKeyFrame::Gone { member: a_id });
    assert!(listed(&env, &bob, &id).await);

    b.close(None).await.unwrap();
    let mut deleted = false;
    for _ in 0..50 {
        if !listed(&env, &bob, &id).await {
            deleted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(deleted, "the channel outlived its last key socket");
}
