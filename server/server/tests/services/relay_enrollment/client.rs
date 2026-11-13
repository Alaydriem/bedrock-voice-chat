//! The preflight exchange from the server's side, against a scripted registry on loopback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::PeerEndpoint;
use bvc_server_lib::services::{EnrollmentError, RelayEnrollmentClient};
use common::structs::relay::enroll::{
    EnrollFrame, EnrollRefuseReason, EnrollVersion, PlayPreflightOutcome,
};
use common::structs::relay::wire::Framing;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::EndpointAddr;
use tempfile::TempDir;
use tokio::sync::Notify;

struct Outcome {
    reply: Result<PlayPreflightOutcome, EnrollmentError>,
    nonce: Option<String>,
    handed_over: bool,
}

// A registry on loopback that replays a script of frames.
struct ScriptedRegistry;

impl ScriptedRegistry {
    const SERVED: Duration = Duration::from_secs(5);

    fn reachable() -> PlayPreflightOutcome {
        PlayPreflightOutcome::Reachable {
            address: "203.0.113.9".to_string(),
            port: 25565,
        }
    }

    fn nonce() -> EnrollFrame {
        EnrollFrame::PlayPreflightNonce {
            nonce: "abc123".to_string(),
        }
    }

    fn result(outcome: PlayPreflightOutcome) -> EnrollFrame {
        EnrollFrame::PlayPreflightResult { outcome }
    }

    async fn read_frame(recv: &mut RecvStream) -> EnrollFrame {
        let mut header = [0u8; Framing::HEADER_LEN];
        recv.read_exact(&mut header).await.expect("header");
        let mut payload = vec![0u8; Framing::payload_len(&header).expect("length")];
        recv.read_exact(&mut payload).await.expect("payload");
        Framing::decode(&payload).expect("frame")
    }

    async fn write_frame(send: &mut SendStream, frame: &EnrollFrame) {
        send.write_all(&Framing::encode(frame).expect("encode"))
            .await
            .expect("write");
    }

    // Answers `Hello` with `Ready { version }`, then answers the request stream with
    // `replies`. After a nonce it waits until the client has handed the nonce over, and
    // records that it did, so the test can tell the nonce reached the caller before the
    // result was read.
    async fn answer(
        conn: Connection,
        version: u16,
        replies: Vec<EnrollFrame>,
        served: Arc<Notify>,
        handed_over: Arc<AtomicBool>,
    ) {
        let Ok((mut send, mut recv)) = conn.accept_bi().await else {
            return;
        };
        Self::read_frame(&mut recv).await;
        Self::write_frame(
            &mut send,
            &EnrollFrame::Ready {
                version: EnrollVersion(version),
            },
        )
        .await;
        send.finish().expect("finish");

        let Ok((mut send, mut recv)) = conn.accept_bi().await else {
            return;
        };
        Self::read_frame(&mut recv).await;
        for frame in &replies {
            Self::write_frame(&mut send, frame).await;
            if matches!(frame, EnrollFrame::PlayPreflightNonce { .. })
                && tokio::time::timeout(Self::SERVED, served.notified())
                    .await
                    .is_ok()
            {
                handed_over.store(true, Ordering::SeqCst);
            }
        }
        send.finish().expect("finish");
        let _ = conn.closed().await;
    }

    async fn exchange(version: u16, replies: Vec<EnrollFrame>) -> Outcome {
        let registry_dir = TempDir::new().expect("tempdir");
        let client_dir = TempDir::new().expect("tempdir");

        let registry_identity =
            NodeIdentity::load_or_create(registry_dir.path().to_str().expect("utf-8 path"))
                .expect("identity");
        let registry = PeerEndpoint::bind_with_alpns(
            &registry_identity,
            None,
            vec![RelayEnrollmentClient::ALPN.to_vec()],
        )
        .await
        .expect("bind");

        let mut addr = EndpointAddr::new(registry.node_id());
        for socket in registry.endpoint().bound_sockets() {
            if socket.is_ipv4() {
                addr = addr.with_ip_addr(std::net::SocketAddr::new(
                    std::net::Ipv4Addr::LOCALHOST.into(),
                    socket.port(),
                ));
            }
        }

        let served = Arc::new(Notify::new());
        let handed_over = Arc::new(AtomicBool::new(false));
        let listening = registry.endpoint().clone();
        let task = tokio::spawn({
            let served = served.clone();
            let handed_over = handed_over.clone();
            async move {
                let incoming = listening.accept().await.expect("incoming");
                let conn = incoming.await.expect("connection");
                Self::answer(conn, version, replies, served, handed_over).await;
            }
        });

        let client_identity =
            NodeIdentity::load_or_create(client_dir.path().to_str().expect("utf-8 path"))
                .expect("identity");
        let client = RelayEnrollmentClient::new(&client_identity, addr, None)
            .await
            .expect("client");

        let seen = Arc::new(Mutex::new(None));
        let reply = client
            .play_preflight(None, 25565, {
                let seen = seen.clone();
                let served = served.clone();
                move |value| {
                    *seen.lock().unwrap() = Some(value);
                    served.notify_one();
                }
            })
            .await;

        let _ = tokio::time::timeout(Self::SERVED, task).await;
        client.close().await;
        registry.close().await;

        let nonce = seen.lock().unwrap().clone();
        Outcome {
            reply,
            nonce,
            handed_over: handed_over.load(Ordering::SeqCst),
        }
    }
}

#[tokio::test]
async fn a_nonce_reaches_the_caller_before_the_result_is_read() {
    let outcome = ScriptedRegistry::exchange(
        3,
        vec![
            ScriptedRegistry::nonce(),
            ScriptedRegistry::result(ScriptedRegistry::reachable()),
        ],
    )
    .await;

    assert_eq!(
        outcome.reply.expect("an outcome"),
        ScriptedRegistry::reachable()
    );
    assert_eq!(outcome.nonce.as_deref(), Some("abc123"));
    assert!(outcome.handed_over);
}

#[tokio::test]
async fn a_result_without_a_nonce_is_returned_and_serves_nothing() {
    let timed_out = PlayPreflightOutcome::TimedOut {
        address: "203.0.113.9".to_string(),
        port: 25565,
    };
    let outcome =
        ScriptedRegistry::exchange(3, vec![ScriptedRegistry::result(timed_out.clone())]).await;

    assert_eq!(outcome.reply.expect("an outcome"), timed_out);
    assert_eq!(outcome.nonce, None);
}

#[tokio::test]
async fn rate_limiting_after_the_nonce_is_an_outcome() {
    let limited = PlayPreflightOutcome::RateLimited {
        retry_after_secs: 120,
    };
    let outcome = ScriptedRegistry::exchange(
        3,
        vec![
            ScriptedRegistry::nonce(),
            ScriptedRegistry::result(limited.clone()),
        ],
    )
    .await;

    assert_eq!(outcome.reply.expect("an outcome"), limited);
    assert_eq!(outcome.nonce.as_deref(), Some("abc123"));
}

#[tokio::test]
async fn a_refusal_carries_its_reason() {
    let outcome = ScriptedRegistry::exchange(
        3,
        vec![EnrollFrame::Refuse {
            reason: EnrollRefuseReason::PreflightRequired,
        }],
    )
    .await;

    let error = outcome.reply.expect_err("a refusal");
    assert_eq!(error.reason(), Some(EnrollRefuseReason::PreflightRequired));
}

#[tokio::test]
async fn another_frame_first_names_both_frames_it_accepts() {
    let outcome = ScriptedRegistry::exchange(
        3,
        vec![EnrollFrame::CheckedIn { nonce: Vec::new() }],
    )
    .await;

    match outcome.reply.expect_err("an unexpected frame") {
        EnrollmentError::Unexpected { expected } => {
            assert_eq!(expected, "PlayPreflightNonce or PlayPreflightResult");
        }
        other => panic!("expected Unexpected, got {other:?}"),
    }
}

#[tokio::test]
async fn a_registry_older_than_play_is_named_before_any_request() {
    let outcome = ScriptedRegistry::exchange(
        2,
        vec![ScriptedRegistry::result(ScriptedRegistry::reachable())],
    )
    .await;

    assert!(matches!(
        outcome.reply,
        Err(EnrollmentError::PlayNotOffered { version: 2 })
    ));
    assert_eq!(outcome.nonce, None);
}

#[tokio::test]
async fn no_observed_address_is_refused_with_its_own_explanation() {
    let outcome = ScriptedRegistry::exchange(
        3,
        vec![EnrollFrame::Refuse {
            reason: EnrollRefuseReason::NoObservedAddress,
        }],
    )
    .await;

    let error = outcome.reply.expect_err("a refusal");
    assert_eq!(error.reason(), Some(EnrollRefuseReason::NoObservedAddress));
    assert!(error.to_string().contains("server.tls.play.address"));
}
