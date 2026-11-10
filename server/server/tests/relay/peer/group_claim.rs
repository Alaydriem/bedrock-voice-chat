use std::collections::HashMap;
use std::sync::Arc;

use bvc_relay::node::PeerTicket;
use bvc_relay::peer::PeerVoice;
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::{GrantTable, LocalClients, PeerIngest};
use bvc_server_lib::services::metrics_service::interaction::RouteRejection;
use bvc_server_lib::services::{ChannelMembershipService, ClientActionService};
use bvc_server_lib::stream::quic::connection::{ConnectionRegistry, RoutedPacket};
use bvc_server_lib::stream::quic::{CacheManager, PacketOrigin, WebhookReceiver};
use common::game_data::Dimension;
use common::structs::control::ClientActionType;
use common::structs::packet::QuicNetworkPacket;
use common::structs::relay::wire::datagram::{GroupScope, VoiceFrame};
use common::{Coordinate, Game, MinecraftPlayer, Orientation, PlayerEnum};
use iroh::{EndpointAddr, PublicKey, SecretKey};
use tokio::sync::mpsc;

use crate::harness::RoutingFixture;

const RANGE: f32 = 16.0;
const WHISPER: f32 = 4.0;
const WORLD: &str = "W1";

// The bridge's speakers are Java players, who hold no BVC client.
struct NoLocals;

impl LocalClients for NoLocals {
    fn has_live_client(&self, _identity: &str) -> bool {
        false
    }
}

// A server with one connected Bedrock listener, a members-only linked channel, and a peer
// granted the world the bridge's speakers are in.
struct Bridged {
    cm: CacheManager,
    registry: Arc<ConnectionRegistry>,
    webhook: WebhookReceiver,
    fanned: mpsc::UnboundedReceiver<(QuicNetworkPacket, PacketOrigin)>,
    ingest: PeerIngest,
    node: PublicKey,
    channel: String,
    bob_rx: mpsc::Receiver<RoutedPacket>,
}

impl Bridged {
    async fn new() -> Self {
        let registry = Arc::new(ConnectionRegistry::new());
        let mut cm = CacheManager::new();
        cm.set_connection_registry(registry.clone());
        let (tx, fanned) = mpsc::unbounded_channel();
        let webhook = WebhookReceiver::new(tx);

        let (bob_tx, bob_rx) = mpsc::channel(8);
        registry
            .try_register(1, "minecraft:Bob".into(), "fp-1".into(), bob_tx)
            .expect("admitted");

        let node = SecretKey::generate().public();
        let mut peers = HashMap::new();
        peers.insert(
            "bridge".to_string(),
            PeerConfig {
                peerlink: PeerTicket::mint(&EndpointAddr::new(node)).expect("mint"),
                worlds: vec![WORLD.to_string()],
                capabilities: vec!["carry_speakers".to_string()],
            },
        );
        let grants = Arc::new(GrantTable::from_config(&peers).expect("valid config"));

        let channel = ChannelMembershipService::create_linked(
            &cm.get_channel_collection(),
            &webhook,
            "Raid".to_string(),
            Game::Minecraft.membership_key("Steve"),
            true,
        )
        .await;

        let mut bridged = Self {
            cm,
            registry,
            webhook,
            fanned,
            ingest: PeerIngest::new(grants, Arc::new(NoLocals)),
            node,
            channel,
            bob_rx,
        };
        bridged.join("Bob").await;
        bridged
    }

    // The join the bridge sends for a player, by in-game name, through the control plane.
    async fn join(&mut self, name: &str) {
        ClientActionService::route_group(
            &ClientActionType::JoinGroup {
                channel: self.channel.clone(),
            },
            &Game::Minecraft.membership_key(name),
            &self.cm.get_channel_collection(),
            &self.webhook,
        )
        .await
        .expect("joined");

        while let Ok((packet, _)) = self.fanned.try_recv() {
            self.cm.process_packet(packet).await.expect("processed");
        }
    }

    // One frame from the bridge for a Java speaker standing beside Bob, claiming the channel.
    async fn speak(&mut self, name: &str) {
        let voice = PeerVoice {
            frame: VoiceFrame {
                speaker: PlayerEnum::Minecraft(MinecraftPlayer {
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
                    world_uuid: None,
                    alternative_identity: None,
                    player_uuid: None,
                    relay_world_uuid: Some(WORLD.to_string()),
                    bridged_voice: false,
                }),
                sample_rate: 48000,
                opus: vec![0xAA],
                timestamp_ms: 0,
                spatial: true,
                jukebox: None,
            },
            group: Some(GroupScope {
                channel: self.channel.clone(),
                members_only: true,
            }),
        };
        let (packet, speaker, group) = self
            .ingest
            .admit_voice(&self.node, voice)
            .expect("admitted");
        let cache = RoutingFixture::player_cache(&[
            speaker.clone(),
            RoutingFixture::player("Bob", 1.0, false),
        ])
        .await;

        self.registry
            .route_audio_frame(
                &packet,
                Some(&speaker),
                group.as_ref(),
                &cache,
                RANGE,
                WHISPER,
            )
            .await;
    }

    fn unverified_claims(&self) -> u64 {
        self.registry
            .route_rejections()
            .get(RouteRejection::GroupClaimUnverified)
    }
}

// The claim is accepted only when the key the ingest stamps on a relayed frame is the key the
// control plane stored the membership under. Each key here comes from the code that makes it
// in production, so the two cannot agree by being written the same way in a test.
#[tokio::test]
async fn a_java_speaker_joined_by_name_has_their_members_only_claim_accepted() {
    let mut bridged = Bridged::new().await;
    bridged.join("Steve").await;

    bridged.speak("Steve").await;

    assert!(
        RoutingFixture::delivered_frame(&mut bridged.bob_rx)
            .await
            .is_some()
    );
    assert_eq!(bridged.unverified_claims(), 0);
}

#[tokio::test]
async fn the_same_claim_before_the_join_reaches_nobody() {
    let mut bridged = Bridged::new().await;

    bridged.speak("Steve").await;

    assert!(
        RoutingFixture::delivered_frame(&mut bridged.bob_rx)
            .await
            .is_none()
    );
    assert_eq!(bridged.unverified_claims(), 1);
}
