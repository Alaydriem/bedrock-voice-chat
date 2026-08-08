use std::sync::{Arc, Mutex};

use bvc_server_lib::services::{ChatRejection, ChatService, ChatSink};
use common::structs::packet::ChatMessagePacket;

/// Stands in for QUIC so the assertions are about routing rather than about transport.
struct RecordingSink {
    delivered: Mutex<Vec<(String, String)>>,
}

impl RecordingSink {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            delivered: Mutex::new(Vec::new()),
        })
    }

    fn worlds(&self) -> Vec<String> {
        self.delivered
            .lock()
            .unwrap()
            .iter()
            .map(|(w, _)| w.clone())
            .collect()
    }

    fn texts(&self) -> Vec<String> {
        self.delivered
            .lock()
            .unwrap()
            .iter()
            .map(|(_, t)| t.clone())
            .collect()
    }
}

impl ChatSink for RecordingSink {
    fn deliver(&self, world_uuid: &str, packet: &ChatMessagePacket) {
        self.delivered
            .lock()
            .unwrap()
            .push((world_uuid.to_string(), packet.text.clone()));
    }
}

// A mod restart whose previous socket has not timed out would otherwise leave two registered:
// every `say` pushed twice and every line reported twice.
#[tokio::test]
async fn a_second_registration_for_a_world_displaces_the_first() {
    let svc = ChatService::new_shared();
    let (tx_a, _rx_a) = tokio::sync::mpsc::channel(8);
    let (tx_b, _rx_b) = tokio::sync::mpsc::channel(8);

    assert!(svc.register("w1".into(), "Survival".into(), tx_a).is_none());
    let displaced = svc.register("w1".into(), "Survival".into(), tx_b);

    assert!(
        displaced.is_some(),
        "the previous socket must be handed back so the caller can close it"
    );
}

#[tokio::test]
async fn availability_follows_registration() {
    let svc = ChatService::new_shared();
    assert!(!svc.is_available("w1"));

    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    svc.register("w1".into(), "Survival".into(), tx);
    assert!(svc.is_available("w1"));

    svc.unregister("w1");
    assert!(!svc.is_available("w1"));
}

#[tokio::test]
async fn the_world_name_from_hello_is_retained_for_the_picker_label() {
    let svc = ChatService::new_shared();
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    svc.register("w1".into(), "Survival".into(), tx);

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
    svc.register("w1".into(), "Survival".into(), tx);

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
    svc.register("w1".into(), "Survival".into(), tx);

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
    svc.register_room(&worlds, "Survival".into(), tx);

    svc.on_app_send("minecraft:Alaydriem", "overworld", "from the app".into())
        .await
        .expect("the send should be accepted");

    assert_eq!(sink.worlds(), worlds, "delivered under every id in the room");
}
