use bvc_server_lib::services::ClientActionService;
use bvc_server_lib::stream::quic::WebhookReceiver;
use bvc_server_lib::stream::quic::connection_registry::{ConnectionRegistry, RoutedPacket};
use bvc_server_lib::stream::quic::{CacheTrait, PlayerPreferenceCache, PlayerStateCache};
use common::Game;
use common::structs::channel::{Channel, ChannelCollection};
use common::structs::control::{ClientAction, ClientActionType, PreferenceKey, QueryState};
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
    registry.register(b"alice".to_vec(), "Alice".to_string(), Game::Minecraft, tx_a);
    registry.register(b"bob".to_vec(), "Bob".to_string(), Game::Minecraft, tx_b);

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

fn reported_state(id: &str) -> QueryState {
    QueryState {
        id: id.to_string(),
        muted: false,
        deafened: true,
        recording: false,
        current_group: None,
    }
}

// The optimistic echo: a DELIVERED self action is folded into the state cache the
// panel polls, so a poll racing the client's debounced confirming report reads the
// post-action value. Untouched fields survive the patch.
#[tokio::test]
async fn delivered_self_action_echoes_into_reported_state() {
    let registry = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    registry.register(b"alice".to_vec(), "Alice".to_string(), Game::Minecraft, tx);
    let player_state = PlayerStateCache::new();
    let preferences = PlayerPreferenceCache::new();
    player_state
        .set("Alice".to_string(), reported_state("Alice"))
        .await;

    let svc = ClientActionService::new();
    let delivered = svc
        .route_self_with_echo(
            &ClientAction {
                id: "Alice".into(),
                action: ClientActionType::SetMuted(true),
            },
            "Alice",
            &registry,
            &player_state,
            &preferences,
        )
        .await;

    assert!(delivered);
    let state = player_state.get(&"Alice".to_string()).await.unwrap();
    assert!(state.muted, "the delivered mute must be visible to the next poll");
    assert!(state.deafened, "fields the action didn't touch must survive the patch");
}

// An UNDELIVERED action must not be echoed: claiming a state change the client
// never received would freeze the panel on a lie until the next real report.
#[tokio::test]
async fn undelivered_self_action_leaves_reported_state_untouched() {
    let registry = ConnectionRegistry::new();
    let player_state = PlayerStateCache::new();
    let preferences = PlayerPreferenceCache::new();
    player_state
        .set("Alice".to_string(), reported_state("Alice"))
        .await;

    let svc = ClientActionService::new();
    let delivered = svc
        .route_self_with_echo(
            &ClientAction {
                id: "Alice".into(),
                action: ClientActionType::SetMuted(true),
            },
            "Alice",
            &registry,
            &player_state,
            &preferences,
        )
        .await;

    assert!(!delivered);
    let state = player_state.get(&"Alice".to_string()).await.unwrap();
    assert!(!state.muted, "no delivery, no echo");
}

// The echo never fabricates a full QueryState from a single field — a player whose
// client has never reported stays "state unavailable" rather than gaining invented
// deafen/record values.
#[tokio::test]
async fn echo_never_fabricates_self_state_for_unreported_player() {
    let registry = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    registry.register(b"alice".to_vec(), "Alice".to_string(), Game::Minecraft, tx);
    let player_state = PlayerStateCache::new();
    let preferences = PlayerPreferenceCache::new();

    let svc = ClientActionService::new();
    svc.route_self_with_echo(
        &ClientAction {
            id: "Alice".into(),
            action: ClientActionType::SetMuted(true),
        },
        "Alice",
        &registry,
        &player_state,
        &preferences,
    )
    .await;

    assert!(
        player_state.get(&"Alice".to_string()).await.is_none(),
        "an unreported player must not gain a fabricated state"
    );
}

// Preference actions upsert (the action carries the whole entry): SetVolume creates
// the entry, SetHeard then patches it without losing the volume.
#[tokio::test]
async fn delivered_preference_actions_upsert_into_preference_cache() {
    let registry = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(8);
    registry.register(b"alice".to_vec(), "Alice".to_string(), Game::Minecraft, tx);
    let player_state = PlayerStateCache::new();
    let preferences = PlayerPreferenceCache::new();
    let svc = ClientActionService::new();

    svc.route_self_with_echo(
        &ClientAction {
            id: "Alice".into(),
            action: ClientActionType::SetVolume {
                target: "Bob".into(),
                volume: 0.5,
            },
        },
        "Alice",
        &registry,
        &player_state,
        &preferences,
    )
    .await;
    svc.route_self_with_echo(
        &ClientAction {
            id: "Alice".into(),
            action: ClientActionType::SetHeard {
                target: "Bob".into(),
                muted: true,
            },
        },
        "Alice",
        &registry,
        &player_state,
        &preferences,
    )
    .await;

    let pref = preferences
        .get(&PreferenceKey::new("Alice", "Bob"))
        .await
        .expect("SetVolume must create the preference");
    assert_eq!(pref.volume, 0.5);
    assert!(pref.muted, "SetHeard must patch the same entry");
}

// The echo mirrors the client's sanitation: a volume outside [0,1] is clamped
// before it can be served back to the panel's 0-100 slider.
#[tokio::test]
async fn echoed_volume_is_clamped_to_client_range() {
    let registry = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    registry.register(b"alice".to_vec(), "Alice".to_string(), Game::Minecraft, tx);
    let player_state = PlayerStateCache::new();
    let preferences = PlayerPreferenceCache::new();
    let svc = ClientActionService::new();

    svc.route_self_with_echo(
        &ClientAction {
            id: "Alice".into(),
            action: ClientActionType::SetVolume {
                target: "Bob".into(),
                volume: 5.0,
            },
        },
        "Alice",
        &registry,
        &player_state,
        &preferences,
    )
    .await;

    let pref = preferences
        .get(&PreferenceKey::new("Alice", "Bob"))
        .await
        .expect("a clamped entry is still written");
    assert_eq!(pref.volume, 1.0, "the served gain never exceeds what the client applies");
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

// Group actions are MOVES: creating a group while in another leaves the old one
// (closing it if emptied) — a player never occupies two groups via this plane.
#[tokio::test]
async fn create_group_moves_actor_out_of_previous_group() {
    let channels = ChannelCollection::new(64);
    let (webhook, mut _rx) = test_webhook();
    let svc = ClientActionService::new();

    let first = svc
        .route_group(
            &ClientActionType::CreateGroup,
            "minecraft:Alice",
            &channels,
            &webhook,
        )
        .await
        .unwrap()
        .unwrap();
    let second = svc
        .route_group(
            &ClientActionType::CreateGroup,
            "minecraft:Alice",
            &channels,
            &webhook,
        )
        .await
        .unwrap()
        .unwrap();

    let memberships = channels.get_player_channels("minecraft:Alice");
    assert_eq!(memberships, vec![second.clone()], "only the new group remains");
    assert!(
        channels.get(&first).await.is_none(),
        "the emptied previous group is closed"
    );
}

#[tokio::test]
async fn join_group_moves_actor_out_of_previous_group() {
    let channels = ChannelCollection::new(64);
    let ch = Channel::new("squad".into(), "minecraft:Owner".into());
    let target = ch.id();
    channels.insert(ch).await;
    // The target must survive Alice's move, so give it another member.
    channels
        .add_player_to_channel("minecraft:Owner", &target)
        .await;
    let (webhook, mut _rx) = test_webhook();
    let svc = ClientActionService::new();

    let own = svc
        .route_group(
            &ClientActionType::CreateGroup,
            "minecraft:Alice",
            &channels,
            &webhook,
        )
        .await
        .unwrap()
        .unwrap();
    svc.route_group(
        &ClientActionType::JoinGroup {
            channel: target.clone(),
        },
        "minecraft:Alice",
        &channels,
        &webhook,
    )
    .await
    .unwrap();

    assert_eq!(
        channels.get_player_channels("minecraft:Alice"),
        vec![target],
        "the join replaced the previous membership"
    );
    assert!(
        channels.get(&own).await.is_none(),
        "the emptied previous group is closed"
    );
}

// A bad share code must not disturb the current group: existence is validated
// before anything is left.
#[tokio::test]
async fn join_unknown_group_keeps_current_membership() {
    let channels = ChannelCollection::new(64);
    let (webhook, mut _rx) = test_webhook();
    let svc = ClientActionService::new();

    let own = svc
        .route_group(
            &ClientActionType::CreateGroup,
            "minecraft:Alice",
            &channels,
            &webhook,
        )
        .await
        .unwrap()
        .unwrap();
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

    assert!(res.is_err());
    assert_eq!(
        channels.get_player_channels("minecraft:Alice"),
        vec![own],
        "a typo'd code must not kick the actor out of their group"
    );
}

// Re-joining the current group is a no-op — a move here would close a solo
// group under its last member and then fail the join.
#[tokio::test]
async fn join_current_group_is_a_noop() {
    let channels = ChannelCollection::new(64);
    let (webhook, mut _rx) = test_webhook();
    let svc = ClientActionService::new();

    let own = svc
        .route_group(
            &ClientActionType::CreateGroup,
            "minecraft:Alice",
            &channels,
            &webhook,
        )
        .await
        .unwrap()
        .unwrap();
    svc.route_group(
        &ClientActionType::JoinGroup {
            channel: own.clone(),
        },
        "minecraft:Alice",
        &channels,
        &webhook,
    )
    .await
    .unwrap();

    assert_eq!(
        channels.get_player_channels("minecraft:Alice"),
        vec![own],
        "the solo group survives a repeat join"
    );
}

// The keying split: ONE actor id must route through BOTH the bare-gamertag
// connection (self delivery) AND the cert-CN membership key (groups).
#[tokio::test]
async fn one_actor_routes_both_self_delivery_and_group_membership() {
    let registry = ConnectionRegistry::new();
    let (tx, mut rx) = mpsc::channel(4);
    registry.register(b"alice".to_vec(), "Alice".to_string(), Game::Minecraft, tx);
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
