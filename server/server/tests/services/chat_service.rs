use std::sync::{Arc, Mutex};

use bvc_server_lib::services::{ChatService, ChatSink};
use common::errors::ChatRejection;
use common::structs::packet::{ChatMessagePacket, ChatRejectedPacket};

/// Stands in for QUIC so the assertions are about routing rather than about transport.
struct RecordingSink {
    delivered: Mutex<Vec<(String, String, Option<String>)>>,
    rejections: Mutex<Vec<(String, ChatRejectedPacket)>>,
}

impl RecordingSink {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            delivered: Mutex::new(Vec::new()),
            rejections: Mutex::new(Vec::new()),
        })
    }

    fn worlds(&self) -> Vec<String> {
        self.delivered
            .lock()
            .unwrap()
            .iter()
            .map(|(w, _, _)| w.clone())
            .collect()
    }

    fn texts(&self) -> Vec<String> {
        self.delivered
            .lock()
            .unwrap()
            .iter()
            .map(|(_, t, _)| t.clone())
            .collect()
    }

    /// Which delivery, if any, was told who sent the line.
    fn authors(&self) -> Vec<Option<String>> {
        self.delivered
            .lock()
            .unwrap()
            .iter()
            .map(|(_, _, a)| a.clone())
            .collect()
    }

    /// Which senders were told their line was refused, and with what.
    fn rejections(&self) -> Vec<(String, ChatRejectedPacket)> {
        self.rejections.lock().unwrap().clone()
    }
}

impl ChatSink for RecordingSink {
    fn deliver(&self, world_uuid: &str, author_identity: Option<&str>, packet: &ChatMessagePacket) {
        self.delivered.lock().unwrap().push((
            world_uuid.to_string(),
            packet.text.clone(),
            author_identity.map(str::to_string),
        ));
    }

    fn deliver_rejection(&self, identity: &str, packet: &ChatRejectedPacket) {
        self.rejections
            .lock()
            .unwrap()
            .push((identity.to_string(), packet.clone()));
    }
}

// A mod restart whose previous socket has not timed out would otherwise leave two registered:
// every `say` pushed twice and every line reported twice.
#[tokio::test]
async fn a_second_registration_for_a_world_displaces_the_first() {
    let svc = ChatService::new_shared();
    let (tx_a, _rx_a) = tokio::sync::mpsc::channel(8);
    let (tx_b, _rx_b) = tokio::sync::mpsc::channel(8);

    assert!(
        svc.register(svc.next_socket_id(), "w1".into(), "Survival".into(), tx_a)
            .is_none()
    );
    let displaced = svc.register(svc.next_socket_id(), "w1".into(), "Survival".into(), tx_b);

    assert!(
        displaced.is_some(),
        "the previous socket must be handed back so the caller can close it"
    );
}

// The caller hands over its only sender, so dropping the displaced one is what ends that
// connection. A retained clone anywhere leaves the displaced socket running unregistered and
// still connected, which is how one world accumulated five live sockets.
#[tokio::test]
async fn dropping_a_displaced_sender_closes_that_sockets_outbound_channel() {
    let svc = ChatService::new_shared();
    let worlds = vec!["overworld".to_string(), "nether".to_string()];

    let (tx_a, mut rx_a) = tokio::sync::mpsc::channel(8);
    svc.register_room(svc.next_socket_id(), &worlds, "Survival".into(), tx_a);

    let (tx_b, _rx_b) = tokio::sync::mpsc::channel(8);
    let displaced = svc.register_room(svc.next_socket_id(), &worlds, "Survival".into(), tx_b);

    for sender in displaced {
        drop(sender);
    }

    assert!(
        rx_a.recv().await.is_none(),
        "the displaced socket must observe its channel close so its loop can end"
    );
}

// A displaced socket tears down under the ids it registered, and it does so after the socket
// that replaced it is already serving them. Releasing by world alone removed the live
// registration: chat then stopped with nothing logged and no frame refused.
#[tokio::test]
async fn a_displaced_sockets_teardown_leaves_the_live_registration_alone() {
    let svc = ChatService::new_shared();

    let (tx_a, _rx_a) = tokio::sync::mpsc::channel(8);
    let first = svc.next_socket_id();
    svc.register(first, "w1".into(), "Survival".into(), tx_a);

    let (tx_b, mut rx_b) = tokio::sync::mpsc::channel(8);
    let second = svc.next_socket_id();
    svc.register(second, "w1".into(), "Survival".into(), tx_b);

    svc.unregister("w1", first);

    assert!(
        svc.is_available("w1"),
        "the live socket must keep the world it registered"
    );
    svc.on_app_send("minecraft:Alaydriem", "w1", "still here".into())
        .await
        .expect("the live socket must still accept a send");
    assert!(rx_b.recv().await.is_some(), "the live socket must receive it");

    svc.unregister("w1", second);
    assert!(!svc.is_available("w1"));
}

#[tokio::test]
async fn availability_follows_registration() {
    let svc = ChatService::new_shared();
    assert!(!svc.is_available("w1"));

    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let id = svc.next_socket_id();
    svc.register(id, "w1".into(), "Survival".into(), tx);
    assert!(svc.is_available("w1"));

    svc.unregister("w1", id);
    assert!(!svc.is_available("w1"));
}

#[tokio::test]
async fn the_world_name_from_hello_is_retained_for_the_picker_label() {
    let svc = ChatService::new_shared();
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    svc.register(svc.next_socket_id(), "w1".into(), "Survival".into(), tx);

    assert_eq!(svc.world_name("w1").as_deref(), Some("Survival"));
}

#[tokio::test]
async fn an_app_send_to_a_world_with_no_channel_is_rejected() {
    let svc = ChatService::new_shared();

    let result = svc
        .on_app_send("minecraft:Alaydriem", "w1", "hello".into())
        .await;

    assert!(matches!(result, Err(ChatRejection::NoChannel)));
}

#[tokio::test]
async fn an_app_send_reaches_the_registered_socket_as_a_say_frame() {
    let svc = ChatService::new_shared();
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    svc.register(svc.next_socket_id(), "w1".into(), "Survival".into(), tx);

    svc.on_app_send("minecraft:Alaydriem", "w1", "hello".into())
        .await
        .expect("the send should be accepted");

    let frame = rx.recv().await.expect("a frame should be pushed");
    assert!(frame.contains("\"t\":\"say\""), "got {frame}");
    assert!(frame.contains("Alaydriem"), "got {frame}");
    assert!(frame.contains("hello"), "got {frame}");
}

// A player in world A must never receive world B's chat. `broadcast_to_all` is not
// world-scoped, which is exactly why the sink is keyed on the world instead.
#[tokio::test]
async fn a_reported_line_is_labelled_with_the_world_it_came_from() {
    let svc = ChatService::new_shared();
    let sink = RecordingSink::new();
    svc.add_sink(sink.clone());

    svc.on_game_chat(&["w1".to_string()], "Petra".into(), "hello".into())
        .await;
    svc.on_game_chat(
        &["w2".to_string()],
        "Juno".into(),
        "different world".into(),
    )
    .await;

    assert_eq!(sink.worlds(), vec!["w1".to_string(), "w2".to_string()]);
    assert_eq!(
        sink.texts(),
        vec!["hello".to_string(), "different world".to_string()]
    );
}

// The app's own send is fanned out at accept time rather than waiting for the mod to echo it,
// because a programmatic broadcast does not fire the mod's chat listener.
#[tokio::test]
async fn an_app_send_is_also_fanned_out_to_clients() {
    let svc = ChatService::new_shared();
    let sink = RecordingSink::new();
    svc.add_sink(sink.clone());
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    svc.register(svc.next_socket_id(), "w1".into(), "Survival".into(), tx);

    svc.on_app_send("minecraft:Alaydriem", "w1", "from the app".into())
        .await
        .expect("the send should be accepted");

    assert_eq!(sink.texts(), vec!["from the app".to_string()]);
}

// Paper and Fabric mint a world id per dimension, and chat is server-wide. A line typed in the
// overworld has to reach somebody standing in the nether, so it is delivered under every id
// the room spans.
#[tokio::test]
async fn a_room_spanning_several_world_ids_delivers_under_each() {
    let svc = ChatService::new_shared();
    let sink = RecordingSink::new();
    svc.add_sink(sink.clone());

    let worlds = vec![
        "overworld".to_string(),
        "nether".to_string(),
        "the_end".to_string(),
    ];
    svc.on_game_chat(&worlds, "Petra".into(), "hello".into())
        .await;

    assert_eq!(sink.worlds(), worlds);
    assert_eq!(sink.texts().len(), 3, "one delivery per id in the room");
}

// An app-sent line has to reach the whole room, exactly as a typed one does. Delivering it
// only under the id the client named leaves anyone in another dimension unable to see it.
#[tokio::test]
async fn an_app_send_reaches_every_id_the_room_spans() {
    let svc = ChatService::new_shared();
    let sink = RecordingSink::new();
    svc.add_sink(sink.clone());

    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let worlds = vec!["overworld".to_string(), "nether".to_string()];
    svc.register_room(svc.next_socket_id(), &worlds, "Survival".into(), tx);

    svc.on_app_send("minecraft:Alaydriem", "overworld", "from the app".into())
        .await
        .expect("the send should be accepted");

    assert_eq!(sink.worlds(), worlds, "delivered under every id in the room");
}

// Delivery is addressed by in-game presence, and the app's sender is often not in game at
// all — that is the case the off-game picker exists for. Without an explicit echo they watch
// their own message disappear into a world that accepted it.
#[tokio::test]
async fn the_author_is_named_once_on_delivery() {
    let svc = ChatService::new_shared();
    let sink = RecordingSink::new();
    svc.add_sink(sink.clone());
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    svc.register(svc.next_socket_id(), "W".into(), "Survival".into(), tx);

    svc.on_app_send("minecraft:Alaydriem", "W", "hello".to_string())
        .await
        .expect("a registered world accepts a line");

    assert_eq!(
        sink.authors(),
        vec![Some("minecraft:Alaydriem".to_string())],
        "the author must be named exactly once so the sink can guarantee their echo"
    );
}

// A line the world reported is already there. Naming an author would ask the sink to echo it
// back to whoever happened to say it in game.
#[tokio::test]
async fn a_reported_line_names_no_author() {
    let svc = ChatService::new_shared();
    let sink = RecordingSink::new();
    svc.add_sink(sink.clone());

    svc.on_game_chat(&["W".to_string()], "Petra".into(), "hi".into())
        .await;

    assert_eq!(sink.authors(), vec![None]);
}

// A rejection the sender never hears is indistinguishable from a message that landed. With the
// client no longer predicting failures, this is the only surface a real one has.
#[tokio::test]
async fn an_unroutable_world_answers_the_sender() {
    let svc = ChatService::new_shared();
    let sink = RecordingSink::new();
    svc.add_sink(sink.clone());

    let outcome = svc
        .on_app_send("minecraft:Alaydriem", "no-such-world", "hello".into())
        .await;

    assert!(outcome.is_err());
    let rejections = sink.rejections();
    assert_eq!(rejections.len(), 1, "told exactly once");
    assert_eq!(rejections[0].0, "minecraft:Alaydriem");
    assert_eq!(rejections[0].1.text, "hello");
    assert!(
        rejections[0].1.reason.contains("no chat channel"),
        "the sender needs the reason, got: {}",
        rejections[0].1.reason
    );
}

// The refusal goes to the sender alone. Everyone else never saw the message.
#[tokio::test]
async fn a_refusal_is_not_fanned_out_to_the_world() {
    let svc = ChatService::new_shared();
    let sink = RecordingSink::new();
    svc.add_sink(sink.clone());

    let _ = svc
        .on_app_send("minecraft:Alaydriem", "no-such-world", "hello".into())
        .await;

    assert!(sink.texts().is_empty(), "nothing may be delivered to a world");
}
