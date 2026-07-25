use bytes::Bytes;
use common::Game;
use common::PlayerEnum;
use common::structs::packet::{QuicNetworkPacket, QuicNetworkPacketData};
use common::traits::player_data::PlayerData;
use dashmap::DashMap;
use moka::future::Cache;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use tokio::sync::mpsc;

use crate::relay::{ObservedCodeHandler, PeerManager, RelayedPacket};
use crate::services::MetricsService;

pub enum RoutedPacket {
    Serialized(Bytes),
}

pub(crate) struct ConnectionEntry {
    pub player_name: String,
    // The game from this connection's mTLS certificate CN. Channel membership keys
    // are `game:gamertag`, so holding it here lets membership be resolved from the
    // authenticated identity instead of from position data the player may not have
    // sent yet.
    pub game: Game,
    pub tx: mpsc::Sender<RoutedPacket>,
    pub connected_at: Instant,
}

pub struct ConnectionRegistry {
    connections: DashMap<Vec<u8>, ConnectionEntry>,
    // player_name (bare gamertag) -> client_id, for O(1) point-to-point delivery
    // (`send_to_player`) without scanning all connections.
    name_index: DashMap<String, Vec<u8>>,
    // player_name -> channel_id (one channel per player)
    player_channel: DashMap<String, String>,
    // Emits connect/disconnect counters + events. Installed after construction,
    // mirroring the peer_manager / observe_handler OnceLock pattern.
    metrics: OnceLock<Arc<MetricsService>>,
    // Consecutive reap sweeps each stale channel-membership key has been absent for.
    // Drives the grace-period reaper (`reap_stale_channels`).
    channel_absent_ticks: DashMap<String, u32>,
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
            name_index: DashMap::new(),
            player_channel: DashMap::new(),
            peer_manager: OnceLock::new(),
            observe_handler: OnceLock::new(),
            metrics: OnceLock::new(),
            channel_absent_ticks: DashMap::new(),
        }
    }

    // Installs the metrics service. Set once; a later install is ignored.
    pub fn set_metrics(&self, metrics: Arc<MetricsService>) {
        let _ = self.metrics.set(metrics);
    }

    fn active_player_count(&self) -> i64 {
        self.connections.len() as i64
    }

    // The set of currently-connected players, by bare gamertag. Channel membership
    // legitimately outlives a QUIC drop (until the reaper runs), so gauges that must
    // reflect *current* usage filter player_channel against this set. player_channel
    // keys are the cert CN (`game:gamertag`); connections store the bare gamertag, so
    // callers match on the post-`:` portion of the key.
    fn live_player_names(&self) -> std::collections::HashSet<String> {
        self.connections
            .iter()
            .map(|e| e.value().player_name.clone())
            .collect()
    }

    fn active_channel_count(&self) -> i64 {
        let live = self.live_player_names();
        let distinct: std::collections::HashSet<String> = self
            .player_channel
            .iter()
            .filter(|e| {
                let bare = e.key().split_once(':').map(|(_, b)| b).unwrap_or(e.key());
                live.contains(bare)
            })
            .map(|e| e.value().clone())
            .collect();
        distinct.len() as i64
    }

    fn players_in_channels(&self) -> i64 {
        let live = self.live_player_names();
        self.player_channel
            .iter()
            .filter(|e| {
                let bare = e.key().split_once(':').map(|(_, b)| b).unwrap_or(e.key());
                live.contains(bare)
            })
            .count() as i64
    }

    // Pushes current gauge values into the metrics service after any change to
    // connections or channel membership, so /metrics + statsd reflect live state
    // without a polling task. No-op until the metrics service is installed.
    fn push_gauges(&self) {
        if let Some(m) = self.metrics.get() {
            m.set_active_players(self.active_player_count());
            m.set_active_channels(self.active_channel_count());
            m.set_players_in_channels(self.players_in_channels());
        }
    }

    // Raw player_channel size, for observing the reaper's effect in tests.
    pub fn channel_membership_count(&self) -> usize {
        self.player_channel.len()
    }

    // Grace-period reaper, called on a low cadence from the main event loop. A channel
    // membership whose player has been absent for REAP_GRACE_SWEEPS consecutive sweeps
    // (past the reconnect window) is purged — bounding player_channel growth without
    // evicting players mid-reconnect.
    pub fn reap_stale_channels(&self) {
        const REAP_GRACE_SWEEPS: u32 = 2;
        let live = self.live_player_names();
        self.channel_absent_ticks
            .retain(|k, _| self.player_channel.contains_key(k));

        let mut purge: Vec<String> = Vec::new();
        for e in self.player_channel.iter() {
            let key = e.key();
            let bare = key.split_once(':').map(|(_, b)| b).unwrap_or(key);
            if live.contains(bare) {
                self.channel_absent_ticks.remove(key);
            } else {
                let mut n = self.channel_absent_ticks.entry(key.clone()).or_insert(0);
                *n += 1;
                if *n >= REAP_GRACE_SWEEPS {
                    purge.push(key.clone());
                }
            }
        }
        for key in purge {
            self.player_channel.remove(&key);
            self.channel_absent_ticks.remove(&key);
        }
        self.push_gauges();
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
        game: Game,
        tx: mpsc::Sender<RoutedPacket>,
    ) {
        tracing::info!(
            "Registering connection for player: {} (connections: {})",
            player_name,
            self.connections.len() + 1
        );
        self.name_index.insert(player_name.clone(), client_id.clone());
        let replaced = self.connections.insert(
            client_id,
            ConnectionEntry {
                player_name,
                game,
                tx,
                connected_at: Instant::now(),
            },
        );
        if let Some(metrics) = self.metrics.get() {
            // A reconnect reusing the same client_id overwrites the prior entry;
            // close out that session first so connect/disconnect counters and
            // session durations stay balanced.
            if let Some(old) = replaced {
                metrics.record_disconnect(old.connected_at.elapsed());
            }
            metrics.record_connect();
        }
        self.push_gauges();
    }

    pub fn unregister(&self, client_id: &[u8]) {
        if let Some((_, entry)) = self.connections.remove(client_id) {
            // Only clear the index if it still points at THIS connection — a
            // reconnect that already reused the name must not be evicted here.
            self.name_index
                .remove_if(&entry.player_name, |_, v| v.as_slice() == client_id);
            self.player_channel.remove(&entry.player_name);
            if let Some(metrics) = self.metrics.get() {
                metrics.record_disconnect(entry.connected_at.elapsed());
            }
            tracing::info!(
                "Unregistered connection for player: {} (connections: {})",
                entry.player_name,
                self.connections.len()
            );
            self.push_gauges();
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

    // Delivers a single packet to one connected player (by bare gamertag) via the
    // O(1) name index. Returns whether a live connection received it; a closed
    // sender is reaped. Used by the control plane to route a ClientBound action to
    // its authenticated actor.
    pub fn send_to_player(&self, player_name: &str, packet: &QuicNetworkPacket) -> bool {
        let client_id = match self.name_index.get(player_name) {
            Some(id) => id.value().clone(),
            None => return false,
        };
        // Clone the sender and drop the DashMap ref before any send/unregister to
        // avoid holding a shard lock across a potential `unregister`.
        let tx = match self.connections.get(&client_id) {
            Some(entry) => entry.value().tx.clone(),
            None => return false,
        };
        let bytes = match packet.to_datagram() {
            Ok(b) => Bytes::from(b),
            Err(e) => {
                tracing::error!("Failed to serialize packet for {}: {}", player_name, e);
                return false;
            }
        };
        match tx.try_send(RoutedPacket::Serialized(bytes)) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => false,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.unregister(&client_id);
                false
            }
        }
    }

    pub fn update_player_channel(&self, player_name: String, channel_id: String) {
        self.player_channel.insert(player_name, channel_id);
        self.push_gauges();
    }

    pub fn remove_player_channel(&self, player_name: &str) {
        self.player_channel.remove(player_name);
        self.push_gauges();
    }

    pub fn remove_channel(&self, channel_id: &str) {
        self.player_channel.retain(|_, v| v != channel_id);
        self.push_gauges();
    }

    // Builds the `game:gamertag` key used to index `player_channel`.
    // The player_channel map is written by the channel event handler using the
    // cert CN, which is always in `game:gamertag` form (e.g. "minecraft:Alice").
    fn channel_key(game: common::structs::game::Game, name: &str) -> String {
        game.membership_key(name)
    }

    // The authenticated game for a live connection, by player name. `None` when the
    // name has no connection — a server-injected sender such as a jukebox, or a
    // player who has already disconnected.
    fn connection_game(&self, player_name: &str) -> Option<Game> {
        let client_id = self.name_index.get(player_name)?.value().clone();
        self.connections
            .get(&client_id)
            .map(|entry| entry.value().game.clone())
    }

    pub async fn route_audio_frame(
        &self,
        packet: &QuicNetworkPacket,
        player_cache: &Arc<Cache<String, PlayerEnum>>,
        broadcast_range: f32,
        deafen_distance: f32,
    ) {
        let route_started = Instant::now();

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

        // The sender's game comes from its authenticated certificate when it has a
        // live connection, so channel membership resolves even before the player has
        // sent a position. Server-injected senders (jukebox, webhook, relayed peer
        // audio) have no connection, so they fall back to the game on the packet.
        let sender_game: Option<Game> = self
            .connection_game(sender_name)
            .or_else(|| sender_player.as_ref().map(|sp| sp.get_game()));

        let sender_channel: Option<String> = sender_game.and_then(|game| {
            let key = Self::channel_key(game, sender_name);
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
        let snapshot: Vec<(Vec<u8>, String, Game, mpsc::Sender<RoutedPacket>)> = self
            .connections
            .iter()
            .map(|entry| {
                (
                    entry.key().clone(),
                    entry.value().player_name.clone(),
                    entry.value().game.clone(),
                    entry.value().tx.clone(),
                )
            })
            .collect();

        let mut dead_keys: Vec<Vec<u8>> = Vec::new();

        for (client_id, recipient_name, recipient_game, tx) in &snapshot {
            if recipient_name == sender_name {
                continue;
            }

            // Position data is required only by the proximity branch below, so a
            // recipient that has not sent one is still eligible for channel audio.
            let recipient_player = player_cache.get(recipient_name).await;

            // The recipient's game comes from its authenticated certificate, which is
            // held on the connection being routed to, so its channel key resolves
            // without position data.
            let recipient_channel: Option<String> = {
                let key = Self::channel_key(recipient_game.clone(), recipient_name);
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
                // Proximity is the only branch that needs coordinates, so both sides
                // must have reported a position to be compared at all.
                let sp = match &sender_player {
                    Some(p) => p,
                    None => continue,
                };
                let rp = match &recipient_player {
                    Some(p) => p,
                    None => continue,
                };

                if sp.get_game() != rp.get_game() {
                    continue;
                }

                let effective_range = if sp.is_deafened() {
                    deafen_distance
                } else {
                    broadcast_range
                };

                if let Err(e) = sp.can_communicate_with(rp, effective_range) {
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
                    if let Some(m) = self.metrics.get() {
                        m.record_audio_route_drop();
                    }
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

        if let Some(m) = self.metrics.get() {
            m.record_audio_route(route_started.elapsed());
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
