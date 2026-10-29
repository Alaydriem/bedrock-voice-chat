use bvc_server_lib::services::AudioPlaybackService;
use bvc_server_lib::stream::quic::connection::ConnectionRegistry;
use bvc_server_lib::stream::quic::{CacheManager, CacheTrait, WebhookReceiver};
use common::consts::audio::WHISPER_CONTROL_TARGET;
use common::structs::control::{PreferenceKey, WhisperPreference};
use common::traits::player_data::PlayerData;
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::packet::PacketSender;
use common::{Coordinate, Game, Orientation, PlayerEnum};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::harness::RoutingFixture;

fn player(name: &str) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
        coordinates: Coordinate {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        whispering: false,
        spectator: false,
        world_uuid: Some("world-abc".to_string()),
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: None,
        bridged_voice: false,
    })
}

fn whisperer(name: &str) -> PlayerEnum {
    match player(name) {
        PlayerEnum::Minecraft(p) => PlayerEnum::Minecraft(MinecraftPlayer {
            whispering: true,
            ..p
        }),
        other => other,
    }
}

async fn choose_whisper(cm: &CacheManager, owner: &str, enabled: bool) {
    cm.preferences()
        .set(
            PreferenceKey::new(owner, WHISPER_CONTROL_TARGET),
            WhisperPreference::for_owner(owner, enabled),
        )
        .await;
}

async fn seed(cm: &CacheManager, speaker: PlayerEnum) {
    cm.players()
        .set(speaker.identity().to_string(), speaker)
        .await;
}

fn playback() -> AudioPlaybackService {
    let (tx, _rx) = mpsc::unbounded_channel();
    AudioPlaybackService::new(
        WebhookReceiver::new(tx),
        ".".to_string(),
        CancellationToken::new(),
        1,
    )
}

#[tokio::test]
async fn a_player_resolves_from_the_position_cache() {
    let cm = CacheManager::new();
    cm.players()
        .set(
            Game::Minecraft.membership_key("Alice").to_string(),
            player("Alice"),
        )
        .await;

    let frame = RoutingFixture::audio_packet(player("Alice"), "minecraft:Alice");

    assert!(cm.resolve_speaker(&frame).await.is_some());
}

// A service name is not a player identity. The injected store owns the lifetime of a playback's
// speaker, and the position cache's presence TTL would lapse part-way through a track.
#[tokio::test]
async fn a_service_resolves_from_the_injected_store() {
    let cm = CacheManager::new();
    let playback = playback();
    playback
        .register_speaker(
            "jukebox-abcd1234".to_string(),
            player("jukebox-abcd1234"),
            Duration::from_secs(600),
        )
        .await;
    cm.set_injected_speakers(&playback);

    let frame = RoutingFixture::audio_packet_from_sender(
        player("jukebox-abcd1234"),
        PacketSender::for_service("jukebox-abcd1234"),
    );

    assert!(cm.resolve_speaker(&frame).await.is_some());
}

// A name nothing registered is not a speaker, and routing must treat that as "no position"
// rather than falling through to the other store and finding somebody else's.
#[tokio::test]
async fn an_unregistered_service_resolves_to_nothing() {
    let cm = CacheManager::new();
    cm.set_injected_speakers(&playback());

    let frame = RoutingFixture::audio_packet_from_sender(
        player("jukebox-never"),
        PacketSender::for_service("jukebox-never"),
    );

    assert!(cm.resolve_speaker(&frame).await.is_none());
}

// A player sender must not resolve out of the injected store, and a service sender must not
// resolve out of the position cache. Keyed the same way, they would find each other's.
#[tokio::test]
async fn the_two_stores_do_not_answer_for_each_other() {
    let cm = CacheManager::new();
    let playback = playback();
    playback
        .register_speaker(
            "minecraft:Alice".to_string(),
            player("Alice"),
            Duration::from_secs(600),
        )
        .await;
    cm.set_injected_speakers(&playback);

    let player_frame = RoutingFixture::audio_packet(player("Alice"), "minecraft:Alice");
    assert!(
        cm.resolve_speaker(&player_frame).await.is_none(),
        "a player must not resolve out of the injected store"
    );

    cm.players()
        .set("jukebox-abcd1234".to_string(), player("jukebox-abcd1234"))
        .await;
    let service_frame = RoutingFixture::audio_packet_from_sender(
        player("jukebox-abcd1234"),
        PacketSender::for_service("jukebox-abcd1234"),
    );
    assert!(
        cm.resolve_speaker(&service_frame).await.is_none(),
        "a service must not resolve out of the position cache"
    );
}

// Off by default: a player who never touched the setting must sound the same crouched as
// standing, whichever source reported the crouch.
#[tokio::test]
async fn a_whisper_without_a_preference_resolves_as_normal_voice() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(!speaker.is_whispering());
}

#[tokio::test]
async fn a_whisper_with_the_preference_on_stays_a_whisper() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;
    choose_whisper(&cm, "minecraft:Alice", true).await;

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(speaker.is_whispering());
}

#[tokio::test]
async fn a_whisper_with_the_preference_off_resolves_as_normal_voice() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;
    choose_whisper(&cm, "minecraft:Alice", false).await;

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(!speaker.is_whispering());
}

// The preference is keyed on its owner. Matching on the target alone would let one player's
// choice shrink everybody's range.
#[tokio::test]
async fn a_preference_for_another_player_does_not_opt_this_one_in() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;
    choose_whisper(&cm, "minecraft:Bob", true).await;

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(!speaker.is_whispering());
}

// No preference on this server is off, whoever the sender is. A peer-link speaker has no
// preference here, so a crouch that arrives with their record is heard as normal voice.
#[tokio::test]
async fn a_relayed_whisper_without_a_preference_here_is_normal_voice() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;

    let identity: common::PlayerIdentity = "minecraft:Alice".parse().expect("identity");
    let frame =
        RoutingFixture::audio_packet_from_sender(whisperer("Alice"), PacketSender::relayed(identity));
    let speaker = cm.resolve_speaker(&frame).await.expect("resolves");

    assert!(!speaker.is_whispering());
}

// End to end through routing: the resolved speaker is what `route_audio_frame` ranges on, so a
// crouching player who never opted in reaches a listener well past the whisper range.
#[tokio::test]
async fn crouching_without_opting_in_reaches_a_listener_at_normal_range() {
    let cm = CacheManager::new();
    seed(&cm, whisperer("Alice")).await;
    let bob = RoutingFixture::player("Bob", 30.0, false);
    let cache = RoutingFixture::player_cache(&[whisperer("Alice"), bob]).await;

    let reg = ConnectionRegistry::new();
    let (bob_tx, mut bob_rx) = mpsc::channel(16);
    reg.try_register(2, "minecraft:Bob".into(), "fp-2".to_string(), bob_tx)
        .expect("admitted");

    let frame = RoutingFixture::audio_packet(whisperer("Alice"), "minecraft:Alice");
    let speaker = cm.resolve_speaker(&frame).await;
    reg.route_audio_frame(&frame, speaker.as_ref(), None, &cache, 50.0, 10.0).await;

    assert!(RoutingFixture::delivered_spatial(&mut bob_rx).await.is_some());
}

mod group_link {
    use std::sync::Arc;

    use bvc_server_lib::services::ChannelMembershipService;
    use bvc_server_lib::stream::quic::connection::ConnectionRegistry;
    use bvc_server_lib::stream::quic::{CacheManager, PacketOrigin, WebhookReceiver};
    use common::Game;
    use common::structs::control::GroupLinkEventKind;
    use common::structs::packet::QuicNetworkPacket;
    use tokio::sync::mpsc;

    // Every channel change the membership service fans, fed through the cache manager the way
    // the webhook loop feeds it.
    async fn apply_fanned(
        cm: &CacheManager,
        rx: &mut mpsc::UnboundedReceiver<(QuicNetworkPacket, PacketOrigin)>,
    ) {
        while let Ok((packet, _)) = rx.try_recv() {
            cm.process_packet(packet).await.expect("processed");
        }
    }

    fn manager() -> (
        CacheManager,
        Arc<ConnectionRegistry>,
        WebhookReceiver,
        mpsc::UnboundedReceiver<(QuicNetworkPacket, PacketOrigin)>,
    ) {
        let registry = Arc::new(ConnectionRegistry::new());
        let mut cm = CacheManager::new();
        cm.set_connection_registry(registry.clone());
        let (tx, rx) = mpsc::unbounded_channel();
        (cm, registry, WebhookReceiver::new(tx), rx)
    }

    #[tokio::test]
    async fn a_join_to_a_members_only_channel_marks_it_for_routing() {
        let (cm, registry, webhook, mut rx) = manager();
        let channels = cm.get_channel_collection();
        let steve = Game::Minecraft.membership_key("Steve");
        let id = ChannelMembershipService::create_linked(
            &channels,
            &webhook,
            "Raid".to_string(),
            steve.clone(),
            true,
        )
        .await;
        ChannelMembershipService::join(&channels, &webhook, &steve, &id).await;

        apply_fanned(&cm, &mut rx).await;

        assert!(registry.is_members_only_channel(&id));
    }

    #[tokio::test]
    async fn channel_changes_reach_the_group_link_feed() {
        let (cm, _registry, webhook, mut rx) = manager();
        let channels = cm.get_channel_collection();
        let steve = Game::Minecraft.membership_key("Steve");
        let id = ChannelMembershipService::create(
            &channels,
            &webhook,
            "Raid".to_string(),
            steve.clone(),
        )
        .await;
        ChannelMembershipService::join(&channels, &webhook, &steve, &id).await;

        apply_fanned(&cm, &mut rx).await;

        let drain = cm.group_link_service(webhook).feed().drain(0);
        let kinds: Vec<GroupLinkEventKind> = drain.events.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![GroupLinkEventKind::Create, GroupLinkEventKind::Join]
        );
        assert_eq!(drain.events[1].player.as_deref(), Some("Steve"));
        assert_eq!(drain.events[1].channel_name.as_deref(), Some("Raid"));
    }

    // A protected group is never linked, so nothing about it may reach the bridge.
    #[tokio::test]
    async fn a_protected_channel_never_reaches_the_feed() {
        let (cm, _registry, webhook, mut rx) = manager();
        let channels = cm.get_channel_collection();
        let steve = Game::Minecraft.membership_key("Steve");
        let id = ChannelMembershipService::create_protected(
            &channels,
            &webhook,
            "Secret".to_string(),
            steve.clone(),
        )
        .await;
        ChannelMembershipService::join(&channels, &webhook, &steve, &id).await;

        apply_fanned(&cm, &mut rx).await;

        assert!(cm.group_link_service(webhook).feed().drain(0).events.is_empty());
    }

    #[tokio::test]
    async fn protecting_a_linked_channel_tells_the_bridge_once() {
        let (cm, _registry, webhook, mut rx) = manager();
        let channels = cm.get_channel_collection();
        let steve = Game::Minecraft.membership_key("Steve");
        let id = ChannelMembershipService::create(
            &channels,
            &webhook,
            "Raid".to_string(),
            steve.clone(),
        )
        .await;
        ChannelMembershipService::join(&channels, &webhook, &steve, &id).await;
        // Applied as each change happens: the bridge saw the group while it was open.
        apply_fanned(&cm, &mut rx).await;
        ChannelMembershipService::protect(&channels, &webhook, &steve, &id)
            .await
            .expect("protected");
        apply_fanned(&cm, &mut rx).await;
        ChannelMembershipService::leave(&channels, &webhook, &steve, &id, false).await;

        apply_fanned(&cm, &mut rx).await;

        let kinds: Vec<GroupLinkEventKind> = cm
            .group_link_service(webhook)
            .feed()
            .drain(0)
            .events
            .iter()
            .map(|e| e.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                GroupLinkEventKind::Create,
                GroupLinkEventKind::Join,
                GroupLinkEventKind::Protected
            ]
        );
    }
}
