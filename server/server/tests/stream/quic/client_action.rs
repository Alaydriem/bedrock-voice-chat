use bvc_server_lib::services::ClientActionService;
use bvc_server_lib::stream::quic::WebhookReceiver;
use bvc_server_lib::stream::quic::connection_registry::{ConnectionRegistry, RoutedPacket};
use common::structs::channel::{Channel, ChannelCollection};
use common::structs::control::{ClientAction, ClientActionType};
use common::structs::packet::{PacketType, QuicNetworkPacket, QuicNetworkPacketData};
use tokio::sync::mpsc;

fn decode(rp: RoutedPacket) -> QuicNetworkPacket {
    match rp {
        RoutedPacket::Serialized(bytes) => QuicNetworkPacket::from_datagram(&bytes).unwrap(),
    }
}

// route_self's contract: it delivers to `actor_name` and ignores the wire
// `action.id`, rewriting it to the actor. (The HTTP route passes the token-
// attributed id AS the actor, so this is defense-in-depth at the service layer,
// not a system-level anti-forgery guarantee — the mod is the trusted attributor.)
#[test]
fn route_self_delivers_to_actor_name_ignoring_wire_id() {
    let registry = ConnectionRegistry::new();
    let (tx_a, mut rx_a) = mpsc::channel(4);
    let (tx_b, mut rx_b) = mpsc::channel(4);
    registry.register(b"alice".to_vec(), "Alice".to_string(), tx_a);
    registry.register(b"bob".to_vec(), "Bob".to_string(), tx_b);

    let svc = ClientActionService::new();
    // The wire id says Bob, but the supplied actor is Alice — routing follows Alice.
    let action = ClientAction {
        id: "Bob".to_string(),
        action: ClientActionType::SetMuted(true),
    };
    let delivered = svc.route_self(&action, "Alice", &registry);

    assert!(delivered, "actor Alice must receive the ClientBound action");
    let pkt = decode(rx_a.try_recv().expect("Alice must receive it"));
    assert_eq!(pkt.packet_type, PacketType::ClientAction);
    match pkt.data {
        QuicNetworkPacketData::ClientAction(p) => {
            assert_eq!(p.action.action, ClientActionType::SetMuted(true));
            assert_eq!(
                p.action.id, "Alice",
                "server rewrites the id to the authenticated actor"
            );
        }
        other => panic!("expected ClientAction, got {other:?}"),
    }
    assert!(
        rx_b.try_recv().is_err(),
        "Bob must NOT receive it despite the forged id"
    );
}

fn test_webhook() -> (WebhookReceiver, mpsc::UnboundedReceiver<QuicNetworkPacket>) {
    let (tx, rx) = mpsc::unbounded_channel();
    (WebhookReceiver::new(tx), rx)
}

#[tokio::test]
async fn join_group_adds_actor_and_fans_event() {
    let channels = ChannelCollection::new(64);
    let ch = Channel::new("squad".into(), "minecraft:Owner".into());
    let id = ch.id();
    channels.insert(ch).await;
    let (webhook, mut rx) = test_webhook();

    let svc = ClientActionService::new();
    svc.route_group(
        &ClientActionType::JoinGroup {
            channel: id.clone(),
        },
        "minecraft:Alice",
        &channels,
        &webhook,
    )
    .await
    .unwrap();

    let ch = channels.get(&id).await.unwrap();
    assert!(ch.players.contains(&"minecraft:Alice".to_string()));
    let pkt = rx.try_recv().expect("a ChannelEvent must be fanned");
    assert_eq!(pkt.packet_type, PacketType::ChannelEvent);
}

#[tokio::test]
async fn join_missing_channel_errors_and_creates_no_membership() {
    let channels = ChannelCollection::new(64);
    let (webhook, mut _rx) = test_webhook();
    let svc = ClientActionService::new();

    let res = svc
        .route_group(
            &ClientActionType::JoinGroup {
                channel: "bogus".into(),
            },
            "minecraft:Alice",
            &channels,
            &webhook,
        )
        .await;

    assert!(
        res.is_err(),
        "joining a nonexistent channel must error, not create phantom membership"
    );
    assert!(
        channels.get_player_channels("minecraft:Alice").is_empty(),
        "no membership may be created for a bogus channel"
    );
}

#[tokio::test]
async fn create_group_returns_id_and_adds_creator() {
    let channels = ChannelCollection::new(64);
    let (webhook, mut _rx) = test_webhook();
    let svc = ClientActionService::new();

    let id = svc
        .route_group(
            &ClientActionType::CreateGroup,
            "minecraft:Alice",
            &channels,
            &webhook,
        )
        .await
        .unwrap()
        .expect("CreateGroup returns the new nanoid");

    let ch = channels.get(&id).await.unwrap();
    assert!(ch.players.contains(&"minecraft:Alice".to_string()));
}

#[tokio::test]
async fn leave_group_removes_and_closes_when_empty() {
    let channels = ChannelCollection::new(64);
    let (webhook, mut _rx) = test_webhook();
    let svc = ClientActionService::new();

    let id = svc
        .route_group(
            &ClientActionType::CreateGroup,
            "minecraft:Solo",
            &channels,
            &webhook,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(channels.get(&id).await.is_some());

    svc.route_group(
        &ClientActionType::LeaveGroup,
        "minecraft:Solo",
        &channels,
        &webhook,
    )
    .await
    .unwrap();
    assert!(
        channels.get(&id).await.is_none(),
        "a group left empty is closed"
    );
}

// The keying split: ONE actor id must route through BOTH the bare-gamertag
// connection (self delivery) AND the cert-CN membership key (groups).
#[tokio::test]
async fn one_actor_routes_both_self_delivery_and_group_membership() {
    let registry = ConnectionRegistry::new();
    let (tx, mut rx) = mpsc::channel(4);
    registry.register(b"alice".to_vec(), "Alice".to_string(), tx);
    let channels = ChannelCollection::new(64);
    let (webhook, mut _wrx) = test_webhook();
    let svc = ClientActionService::new();

    let delivered = svc.route_self(
        &ClientAction {
            id: "Alice".into(),
            action: ClientActionType::SetMuted(true),
        },
        "Alice",
        &registry,
    );
    assert!(delivered, "self action reaches the bare-gamertag connection");
    assert!(rx.try_recv().is_ok());

    let id = svc
        .route_group(
            &ClientActionType::CreateGroup,
            "minecraft:Alice",
            &channels,
            &webhook,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(
        channels
            .get(&id)
            .await
            .unwrap()
            .players
            .contains(&"minecraft:Alice".to_string()),
        "group membership uses the cert-CN key for the same actor"
    );
}
