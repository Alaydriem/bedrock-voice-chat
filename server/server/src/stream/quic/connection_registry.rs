use bytes::Bytes;
use common::PlayerEnum;
use common::structs::packet::{QuicNetworkPacket, QuicNetworkPacketData};
use common::traits::player_data::PlayerData;
use dashmap::DashMap;
use moka::future::Cache;
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;

use crate::relay::{ObservedCodeHandler, PeerManager, RelayedPacket};

pub(crate) enum RoutedPacket {
    Serialized(Bytes),
}

pub(crate) struct ConnectionEntry {
    pub player_name: String,
    pub tx: mpsc::Sender<RoutedPacket>,
}

pub(crate) struct ConnectionRegistry {
    connections: DashMap<Vec<u8>, ConnectionEntry>,
    // player_name -> channel_id (one channel per player)
    player_channel: DashMap<String, String>,
    // Optional cross-server relay fan-out. When present, LOCAL-origin packets
    // are forwarded to peer servers sharing the sender's relay world. Packets
    // that arrived FROM a peer are not routed through here (single-hop); the
    // relay ingest path publishes them straight to the broadcast loop. Installed
    // after the registry is wired into the QUIC manager and cache.
    peer_manager: OnceLock<Arc<PeerManager>>,
    // Optional asker-side observe handler (Flow 1). When present, a local
    // client's `PeerPresenceObserved` report is redeemed against the offering
    // minter to establish the peer link. Installed alongside the relay manager.
    observe_handler: OnceLock<Arc<dyn ObservedCodeHandler>>,
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            player_channel: DashMap::new(),
            peer_manager: OnceLock::new(),
            observe_handler: OnceLock::new(),
        }
    }

    // Installs the cross-server relay manager. Set once; a later install is
    // ignored.
    pub fn set_peer_manager(&self, peer_manager: Arc<PeerManager>) {
        let _ = self.peer_manager.set(peer_manager);
    }

    // Installs the asker-side observe handler. Set once; a later install is
    // ignored.
    pub fn set_observe_handler(&self, handler: Arc<dyn ObservedCodeHandler>) {
        let _ = self.observe_handler.set(handler);
    }

    // Routes a `!bvcp` code a local client observed in the realm to the observe
    // handler (the asker side of Flow 1). No-op when no handler is wired.
    pub fn on_peer_presence_observed(&self, token: String) {
        if let Some(handler) = self.observe_handler.get() {
            handler.on_observed(token);
        }
    }

    // A local client observed a peer `!bvca` announce in the realm. Record the
    // peer endpoint as live for the observer's world so the offer/forward paths
    // can reach it — the decentralized replacement for relay lookup.
    pub fn on_peer_announce_observed(&self, hashed_world: String, endpoint: String) {
        let Some(peer_manager) = self.peer_manager.get() else {
            return;
        };
        let ep = match endpoint.rsplit_once(':') {
            Some((host, port)) => common::structs::relay::RelayEndpoint {
                host: host.to_string(),
                port: port.parse().unwrap_or(0),
                primary: false,
            },
            None => return,
        };
        if ep.port == 0 {
            return;
        }
        peer_manager.observe_announced_peer(&hashed_world, ep, std::time::Instant::now());
    }

    // The installed relay manager, if any. Used by the QUIC input path to route
    // inbound `PeerPresenceObserved` reports from local clients.
    pub fn peer_manager(&self) -> Option<&Arc<PeerManager>> {
        self.peer_manager.get()
    }

    // Forwards a LOCAL-origin packet to peer servers sharing the sender's relay
    // world. No-op when no relay is wired or the sender carries no
    // `relay_world_uuid`. Off the hot path semantics are preserved by the
    // manager (bounded `try_send`, drop-on-full).
    pub fn forward_local_to_peers(&self, packet: &QuicNetworkPacket) {
        let peer_manager = match self.peer_manager.get() {
            Some(pm) => pm,
            None => return,
        };

        let world = match Self::relay_world_of(packet) {
            Some(w) => w,
            None => return,
        };

        let relayed = RelayedPacket::local(packet.clone());
        peer_manager.forward_local(&relayed, &world);
    }

    // Extracts the sender's `relay_world_uuid` from an audio packet, if any.
    fn relay_world_of(packet: &QuicNetworkPacket) -> Option<String> {
        if let QuicNetworkPacketData::AudioFrame(af) = &packet.data {
            if let Some(sender) = &af.sender {
                if let Some(mc) = sender.as_minecraft() {
                    return mc.relay_world_uuid.clone();
                }
            }
        }
        None
    }

    pub fn register(
        &self,
        client_id: Vec<u8>,
        player_name: String,
        tx: mpsc::Sender<RoutedPacket>,
    ) {
        tracing::info!(
            "Registering connection for player: {} (connections: {})",
            player_name,
            self.connections.len() + 1
        );
        self.connections
            .insert(client_id, ConnectionEntry { player_name, tx });
    }

    pub fn unregister(&self, client_id: &[u8]) {
        if let Some((_, entry)) = self.connections.remove(client_id) {
            self.player_channel.remove(&entry.player_name);
            tracing::info!(
                "Unregistered connection for player: {} (connections: {})",
                entry.player_name,
                self.connections.len()
            );
        }
    }

    pub fn broadcast_to_all(&self, packet: QuicNetworkPacket) {
        let bytes = match packet.to_datagram() {
            Ok(bytes) => Bytes::from(bytes),
            Err(e) => {
                tracing::error!("Failed to serialize broadcast: {}", e);
                return;
            }
        };

        let mut dead_keys: Vec<Vec<u8>> = Vec::new();

        for entry in self.connections.iter() {
            match entry
                .value()
                .tx
                .try_send(RoutedPacket::Serialized(bytes.clone()))
            {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    tracing::debug!(
                        "Dropping broadcast packet for player {} (channel full)",
                        entry.value().player_name,
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    dead_keys.push(entry.key().clone());
                }
            }
        }

        for key in dead_keys {
            self.unregister(&key);
        }
    }

    pub fn update_player_channel(&self, player_name: String, channel_id: String) {
        self.player_channel.insert(player_name, channel_id);
    }

    pub fn remove_player_channel(&self, player_name: &str) {
        self.player_channel.remove(player_name);
    }

    pub fn remove_channel(&self, channel_id: &str) {
        self.player_channel.retain(|_, v| v != channel_id);
    }

    // Builds the `game:gamertag` key used to index `player_channel`.
    // The player_channel map is written by the channel event handler using the
    // cert CN, which is always in `game:gamertag` form (e.g. "minecraft:Alice").
    fn channel_key(game: common::structs::game::Game, name: &str) -> String {
        format!("{}:{}", game.as_str(), name)
    }

    pub async fn route_audio_frame(
        &self,
        packet: &QuicNetworkPacket,
        player_cache: &Arc<Cache<String, PlayerEnum>>,
        broadcast_range: f32,
        deafen_distance: f32,
    ) {
        let sender_name = match &packet.owner {
            Some(owner) => &owner.name,
            None => return,
        };

        let audio_frame = match &packet.data {
            QuicNetworkPacketData::AudioFrame(af) => af,
            _ => return,
        };

        // Resolve sender's PlayerEnum once up-front so the game is available
        // both for the channel-key derivation and for the proximity branch.
        // player_cache is keyed by bare gamertag; audio_frame.sender carries the
        // full PlayerEnum when the client has already sent a position packet.
        let sender_player: Option<PlayerEnum> = match &audio_frame.sender {
            Some(player) => Some(player.clone()),
            None => player_cache.get(sender_name).await,
        };

        // Derive the channel-membership key for the sender only when a game is
        // known. Falls back to None (proximity-only) when the sender has not yet
        // appeared in the position cache.
        let sender_channel: Option<String> = sender_player.as_ref().and_then(|sp| {
            let key = Self::channel_key(sp.get_game(), sender_name);
            self.player_channel.get(&key).map(|r| r.clone())
        });

        let original_spatial = audio_frame.spatial;
        let has_sender = audio_frame.sender.is_some();

        tracing::debug!(
            "route_audio_frame: sender={} original_spatial={:?} has_sender={} sender_channel={:?}",
            sender_name,
            original_spatial,
            has_sender,
            sender_channel,
        );

        // Pre-build serialized variants (single clone, mutate in-place between serializations)
        let mut p = packet.clone();

        let bytes_spatial: Option<Bytes> = {
            if let QuicNetworkPacketData::AudioFrame(ref mut af) = p.data {
                af.spatial = Some(true);
            }
            p.to_datagram().ok().map(Bytes::from)
        };

        let bytes_channel: Option<Bytes> = {
            if let QuicNetworkPacketData::AudioFrame(ref mut af) = p.data {
                af.spatial = Some(false);
            }
            p.to_datagram().ok().map(Bytes::from)
        };

        if bytes_spatial.is_none() && bytes_channel.is_none() {
            return;
        }

        // Snapshot connections to release DashMap shard locks before any .await
        let snapshot: Vec<(Vec<u8>, String, mpsc::Sender<RoutedPacket>)> = self
            .connections
            .iter()
            .map(|entry| {
                (
                    entry.key().clone(),
                    entry.value().player_name.clone(),
                    entry.value().tx.clone(),
                )
            })
            .collect();

        let mut dead_keys: Vec<Vec<u8>> = Vec::new();

        for (client_id, recipient_name, tx) in &snapshot {
            if recipient_name == sender_name {
                continue;
            }

            // Resolve the recipient's PlayerEnum; needed both for the channel-key
            // derivation and for the proximity spatial check below.
            let recipient_player = match player_cache.get(recipient_name).await {
                Some(player) => player,
                None => continue,
            };

            // Build channel-membership key for the recipient using their game
            // prefix, matching the cert-CN format written by the channel handler.
            let recipient_channel: Option<String> = {
                let key = Self::channel_key(recipient_player.get_game(), recipient_name);
                self.player_channel.get(&key).map(|r| r.clone())
            };

            // Channel membership is cross-game by design: a channel id is shared
            // across games, so `minecraft:Bob` and `hytale:Carol` in the same
            // channel hear each other. Only the fallback proximity path below is
            // gated to same-game (different games have unrelated coordinate
            // spaces, so spatial routing between them is meaningless).
            let in_same_channel = match (&sender_channel, &recipient_channel) {
                (Some(sc), Some(rc)) => sc == rc,
                _ => false,
            };

            let bytes_to_send = if in_same_channel {
                tracing::debug!(
                    "route_audio_frame: {} -> {} IN_CHANNEL spatial={:?}",
                    sender_name,
                    recipient_name,
                    original_spatial,
                );
                // Same-channel members always receive the non-spatial variant so
                // the client skips distance-based volume attenuation. Without this,
                // a spatial=true packet would be zeroed by calculate_spatial_audio_data
                // when members are far apart, defeating the channel-bypass entirely.
                match &bytes_channel {
                    Some(b) => b,
                    None => continue,
                }
            } else {
                let sp = match &sender_player {
                    Some(p) => p,
                    None => continue,
                };

                if sp.get_game() != recipient_player.get_game() {
                    continue;
                }

                let effective_range = if sp.is_deafened() {
                    deafen_distance
                } else {
                    broadcast_range
                };

                if let Err(e) = sp.can_communicate_with(&recipient_player, effective_range) {
                    tracing::debug!(
                        "Audio packet {} -> {} rejected: {}",
                        sender_name,
                        recipient_name,
                        e
                    );
                    continue;
                }

                // Some(false) is rejected outside channels
                match original_spatial {
                    Some(false) => continue,
                    Some(true) | None => match &bytes_spatial {
                        Some(b) => b,
                        None => continue,
                    },
                }
            };

            match tx.try_send(RoutedPacket::Serialized(bytes_to_send.clone())) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    tracing::debug!(
                        "Dropping audio packet for player {} (channel full)",
                        recipient_name,
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    dead_keys.push(client_id.clone());
                }
            }
        }

        for key in dead_keys {
            self.unregister(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::{AlwaysProven, PeerTable, RelayIngestSink, RelayedPacket as _RelayedPacket};
    use common::game_data::Dimension;
    use common::players::MinecraftPlayer;
    use common::structs::packet::{AudioFramePacket, PacketOwner, PacketType};
    use common::structs::relay::RelayEndpoint;
    use common::{Coordinate, Orientation};
    use std::time::Instant;

    struct NoopSink;
    #[async_trait::async_trait]
    impl RelayIngestSink for NoopSink {
        async fn publish(&self, _packet: QuicNetworkPacket) {}
    }

    fn registry_with_takeable_peer(
        world: &str,
        peer: RelayEndpoint,
    ) -> (ConnectionRegistry, Arc<PeerManager>) {
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec![world.to_string()]);
        table.set_world_peers(world, vec![peer.clone()]);
        let mgr = Arc::new(PeerManager::new(
            ep("self", 1),
            table,
            Arc::new(NoopSink),
            Arc::new(AlwaysProven),
        ));
        mgr.register_inbound(&PeerManager::endpoint_key(&peer), Instant::now());
        let reg = ConnectionRegistry::new();
        reg.set_peer_manager(mgr.clone());
        (reg, mgr)
    }

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.into(),
            port,
            primary: false,
        }
    }

    fn mc(relay_world: Option<&str>) -> PlayerEnum {
        PlayerEnum::Minecraft(MinecraftPlayer {
            name: "alice".into(),
            coordinates: Coordinate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: relay_world.map(String::from),
        })
    }

    fn audio_packet(sender: Option<PlayerEnum>) -> QuicNetworkPacket {
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            owner: Some(PacketOwner {
                name: "alice".into(),
                client_id: vec![1, 2, 3],
            }),
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![5, 5, 5],
                48000,
                sender,
                Some(true),
            )),
        }
    }

    fn registry_with_peer(world: &str, peers: Vec<RelayEndpoint>) -> ConnectionRegistry {
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec![world.to_string()]);
        table.set_world_peers(world, peers.clone());
        let mgr = PeerManager::new(
            ep("self", 1),
            table,
            Arc::new(NoopSink),
            Arc::new(AlwaysProven),
        );
        for p in &peers {
            mgr.register_inbound(&PeerManager::endpoint_key(p), Instant::now());
        }
        let reg = ConnectionRegistry::new();
        reg.set_peer_manager(Arc::new(mgr));
        reg
    }

    #[test]
    fn relay_world_extracted_from_audio_sender() {
        let p = audio_packet(Some(mc(Some("W1"))));
        assert_eq!(
            ConnectionRegistry::relay_world_of(&p),
            Some("W1".to_string())
        );
    }

    #[test]
    fn no_relay_world_when_sender_absent() {
        let p = audio_packet(None);
        assert_eq!(ConnectionRegistry::relay_world_of(&p), None);
    }

    #[test]
    fn forward_local_is_noop_without_peer_manager() {
        let reg = ConnectionRegistry::new();
        // must not panic; simply does nothing
        reg.forward_local_to_peers(&audio_packet(Some(mc(Some("W1")))));
    }

    #[test]
    fn forward_local_enqueues_for_world_peer() {
        let reg = registry_with_peer("W1", vec![ep("z", 9)]);
        // sanity: the wired manager would forward one copy for a local packet
        let pm = reg.peer_manager().unwrap();
        let local = _RelayedPacket::local(audio_packet(Some(mc(Some("W1")))));
        assert_eq!(pm.forward_local(&local, "W1"), 1);
    }

    // A server-originated AudioFrame (e.g. jukebox playback) whose sender
    // carries a relay_world_uuid must be forwarded to the peer's outbound queue
    // when forward_local_to_peers is called.
    #[tokio::test]
    async fn server_originated_audio_frame_with_relay_world_reaches_peer_queue() {
        let peer = ep("peer", 7);
        let peer_key = PeerManager::endpoint_key(&peer);
        let (reg, mgr) = registry_with_takeable_peer("W1", peer);
        let mut rx = mgr
            .take_outbound_receiver(&peer_key)
            .expect("peer link must expose its outbound receiver");

        reg.forward_local_to_peers(&audio_packet(Some(mc(Some("W1")))));

        let got = rx
            .try_recv()
            .expect("forwarded packet must arrive on peer queue");
        assert_eq!(got.packet.packet_type, PacketType::AudioFrame);
    }

    // A server-originated AudioFrame with no relay_world_uuid (non-jukebox,
    // non-relay) must NOT be forwarded to any peer queue.
    #[tokio::test]
    async fn server_originated_audio_frame_without_relay_world_skips_peers() {
        let peer = ep("peer", 8);
        let peer_key = PeerManager::endpoint_key(&peer);
        let (reg, mgr) = registry_with_takeable_peer("W1", peer);
        let mut rx = mgr
            .take_outbound_receiver(&peer_key)
            .expect("peer link must expose its outbound receiver");

        reg.forward_local_to_peers(&audio_packet(None));

        assert!(
            rx.try_recv().is_err(),
            "packet without relay_world_uuid must not be forwarded to peers"
        );
    }
}
