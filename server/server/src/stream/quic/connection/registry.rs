use common::PlayerEnum;
use common::net::PositionCadence;
use common::structs::packet::{
    PacketSender, PacketType, PlayerDataPacket, QuicNetworkPacket, QuicNetworkPacketData,
    ServerErrorPacket, ServerErrorType,
};
use common::traits::player_data::PlayerData;
use dashmap::DashMap;
use moka::future::Cache;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use super::{AtCapacity, CapacityPolicy, ConnectionEntry, ConnectionSequence, RoutedPacket};
use crate::services::MetricsService;
use crate::services::metrics_service::interaction::InteractionRoute;
use crate::services::metrics_service::interaction::InteractionTracker;
use crate::stream::quic::log_throttle::LogThrottle;
use crate::stream::session::WebSocketDeviceId;

const OVERSIZED_BROADCAST_LOG_INTERVAL: Duration = Duration::from_secs(30);

const CAPACITY_REFUSAL_LOG_INTERVAL: Duration = Duration::from_secs(30);

pub struct ConnectionRegistry {
    // Keyed on the QUIC connection id, which the server mints at accept. It is unforgeable and
    // unique per connection, which is what lets one player hold two of them.
    connections: DashMap<u64, ConnectionEntry>,
    // canonical identity -> connection id, for O(1) point-to-point delivery
    // (`send_to_player`) without scanning all connections.
    name_index: DashMap<Arc<str>, u64>,
    // canonical identity -> channel_id (one channel per player)
    player_channel: DashMap<Arc<str>, Arc<str>>,
    // certificate fingerprint -> connection id. Revocation addresses a live session by the
    // credential it was opened with, so one identity holding two connections on two
    // certificates loses only the revoked one.
    fingerprint_index: DashMap<String, u64>,
    // Emits connect/disconnect counters + events. Installed after construction
    // rather than at build time.
    metrics: OnceLock<Arc<MetricsService>>,
    // The peer plane, when any peer is declared. Absent is the common case: a
    // server with no `peer` block binds no peer socket at all.
    peer_plane: OnceLock<Arc<crate::relay::PeerPlane>>,
    // Consecutive reap sweeps each stale channel-membership key has been absent for.
    // Drives the grace-period reaper (`reap_stale_channels`).
    channel_absent_ticks: DashMap<Arc<str>, u32>,
    // Guards the broadcast serialization-failure log. The inputs that cause a
    // failure recur every tick, so this site would otherwise emit at the
    // source's full rate.
    oversized_broadcast_log: LogThrottle,
    // Last time each speaker's PlayerEnum rode an outbound audio envelope. Keyed on the
    // stamped sender identity so injected and relayed speakers share the cadence with
    // local ones. Entries for departed speakers are reaped by reap_stale_channels.
    sender_attach: DashMap<Arc<str>, Instant>,
    // Identities that departed and may still return, with the moment they left. A
    // reservation counts against the limit, which is what makes it a held slot rather than
    // a hint; it is displaced oldest-first when a new identity has nowhere else to go, so a
    // server that emptied cleanly never refuses everyone for the length of the grace.
    reservations: DashMap<Arc<str>, Instant>,
    // Absent means unlimited, which is every deployment that never configured a limit.
    capacity: OnceLock<CapacityPolicy>,
    // Serializes the admission decision. Without it two concurrent handshakes both read a
    // count one below the limit and both proceed. Taken once per session, never per packet.
    admission: Mutex<()>,
    // Guards the refusal log. A client in its reconnect backoff returns to a full server
    // repeatedly, and every attempt reaches this site.
    capacity_log: LogThrottle,
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
            fingerprint_index: DashMap::new(),
            metrics: OnceLock::new(),
            peer_plane: OnceLock::new(),
            channel_absent_ticks: DashMap::new(),
            oversized_broadcast_log: LogThrottle::new(OVERSIZED_BROADCAST_LOG_INTERVAL),
            sender_attach: DashMap::new(),
            reservations: DashMap::new(),
            capacity: OnceLock::new(),
            admission: Mutex::new(()),
            capacity_log: LogThrottle::new(CAPACITY_REFUSAL_LOG_INTERVAL),
        }
    }

    // Whether this frame carries the speaker's PlayerEnum. True consumes the slot: the
    // timestamp advances, so the next interval is measured from this frame.
    pub fn sender_attach_due(&self, identity: &str, now: Instant) -> bool {
        if let Some(mut last) = self.sender_attach.get_mut(identity) {
            if now.duration_since(*last) < PositionCadence::INTERVAL {
                return false;
            }
            *last = now;
            return true;
        }
        self.sender_attach.insert(Arc::from(identity), now);
        true
    }

    // Whether the next frame from this speaker will attach their state, without spending the
    // interval. `sender_attach_due` is the one that consumes it, and the two must not both be
    // called for one frame: a gate that consumed the interval would leave the egress with
    // nothing to attach, and no listener would ever receive a position.
    pub fn sender_attach_pending(&self, identity: &str, now: Instant) -> bool {
        match self.sender_attach.get(identity) {
            Some(last) => now.duration_since(*last) >= PositionCadence::INTERVAL,
            None => true,
        }
    }

    // Installs the metrics service. Set once; a later install is ignored.
    pub fn set_metrics(&self, metrics: Arc<MetricsService>) {
        let _ = self.metrics.set(metrics);
    }

    // Installs the capacity policy. Set once; a later install is ignored.
    pub fn set_capacity(&self, policy: CapacityPolicy) {
        let _ = self.capacity.set(policy);
    }

    // Held reservations, for observing the grace and the sweep in tests.
    pub fn reservation_count(&self) -> usize {
        self.reservations.len()
    }

    // Installs the peer plane. Set once; a later install is ignored.
    pub fn set_peer_plane(&self, plane: Arc<crate::relay::PeerPlane>) {
        let _ = self.peer_plane.set(plane);
    }

    // The peer plane, for callers that need the endpoint rather than the routing.
    //
    // `None` means this server declares no peers, which is the ordinary case and
    // not a fault: a server nobody bridges to never binds one.
    pub fn peer_plane(&self) -> Option<Arc<crate::relay::PeerPlane>> {
        self.peer_plane.get().cloned()
    }

    // Forwards a LOCAL-origin packet to peers granted the sender's relay world.
    //
    // A no-op when no peer is declared or the packet carries no relay world.
    //
    // Peer-origin packets must never be passed here. They reach the same broadcast
    // loop local ones do — the plane's sink feeds it — so the loop tags each packet
    // with a `PacketOrigin` and calls this only for local ones. That tag is what
    // keeps relay single-hop; without it a peer's frame is returned to the peer that
    // sent it, and the speaker hears themselves.
    /// `speaker` is the sender's player, resolved by the caller. A frame whose speaker is
    /// unknown cannot be scoped to a relay world, so it does not leave.
    pub fn forward_local_to_peers(&self, packet: &QuicNetworkPacket, speaker: Option<&PlayerEnum>) {
        if let (Some(plane), Some(speaker)) = (self.peer_plane.get(), speaker) {
            plane.forward_local(packet, speaker);
        }
    }

    // Distinct connected players, not raw connection entries. A connection id is minted
    // fresh per connection, so a reconnecting player registers a second entry that
    // lives until the stale one is reaped. Counting entries double-counts them —
    // transient for the active gauge, but permanent for the peak high-water mark,
    // which only falls at the UTC day boundary.
    fn active_player_count(&self) -> i64 {
        self.live_identities().len() as i64
    }

    // The set of currently-connected players, by canonical identity. Channel membership
    // legitimately outlives a QUIC drop (until the reaper runs), so gauges that must
    // reflect *current* usage filter player_channel against this set. Both sides are keyed
    // on the same form, so the comparison is a plain lookup.
    fn live_identities(&self) -> std::collections::HashSet<Arc<str>> {
        self.connections
            .iter()
            .map(|e| e.value().identity.clone())
            .collect()
    }

    fn active_channel_count(&self) -> i64 {
        let live = self.live_identities();
        let distinct: std::collections::HashSet<Arc<str>> = self
            .player_channel
            .iter()
            .filter(|e| live.contains(e.key()))
            .map(|e| e.value().clone())
            .collect();
        distinct.len() as i64
    }

    fn players_in_channels(&self) -> i64 {
        let live = self.live_identities();
        self.player_channel
            .iter()
            .filter(|e| live.contains(e.key()))
            .count() as i64
    }

    // Pushes current gauge values into the metrics service after any change to
    // connections or channel membership, so /metrics + statsd reflect live state
    // without a polling task. No-op until the metrics service is installed.
    fn push_gauges(&self) {
        if let Some(m) = self.metrics.get() {
            m.set_active_players(self.active_player_count());
            m.push_peak_players();
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
        let live = self.live_identities();
        self.channel_absent_ticks
            .retain(|k, _| self.player_channel.contains_key(k.as_ref()));

        let mut purge: Vec<Arc<str>> = Vec::new();
        for e in self.player_channel.iter() {
            let key = e.key();
            if live.contains(key) {
                self.channel_absent_ticks.remove(key.as_ref());
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
        // A speaker silent for minutes needs no cadence slot; their next frame re-creates
        // one and attaches immediately, which is exactly the desired first-frame behavior.
        self.sender_attach
            .retain(|_, last| last.elapsed() < Duration::from_secs(300));
        // The sweep bounds the map; admission enforces the grace itself, so a reservation
        // is never honoured past it merely because no sweep has run.
        if let Some(policy) = self.capacity.get() {
            self.expire_reservations(policy.grace());
        }
        self.push_gauges();
    }

    /// Registers this session, unless the server is already at its configured capacity.
    ///
    /// The count is derived from `connections`, the same map that answers
    /// `live_identities()`, so there is no second source of truth to drift from it.
    pub fn try_register(
        &self,
        device: u64,
        identity: Arc<str>,
        fingerprint: String,
        tx: mpsc::Sender<RoutedPacket>,
    ) -> Result<(), AtCapacity> {
        // Poisoning here only means some other caller panicked mid-admission. Losing the
        // serialization is worse than proceeding: without it, admission stops entirely.
        let _admission = match self.admission.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(policy) = self.capacity.get()
            && !policy.is_unlimited()
            && !self.admits(&identity, policy)
        {
            if let Some(metrics) = self.metrics.get() {
                metrics.record_capacity_refusal();
            }
            if let Some(suppressed) = self.capacity_log.should_log() {
                tracing::warn!(
                    "Refusing {}: at capacity ({} connections), {} further refusals suppressed",
                    identity,
                    policy.limit(),
                    suppressed
                );
            }
            return Err(AtCapacity {
                limit: policy.limit(),
            });
        }

        self.reservations.remove(&identity);
        self.insert_connection(device, identity, fingerprint, tx);
        Ok(())
    }

    // The admission decision, with the admission lock held.
    fn admits(&self, identity: &Arc<str>, policy: &CapacityPolicy) -> bool {
        let live = self.live_identities();

        // A reconnect, or one identity on two devices. The slot is already theirs.
        if live.contains(identity) {
            return true;
        }

        // Expiry is enforced here rather than left to the sweep: a reservation is stale the
        // moment it ages past the grace, whether or not a sweep has run since.
        self.expire_reservations(policy.grace());

        if self.reservations.contains_key(identity) {
            return true;
        }

        if (live.len() + self.reservations.len()) < policy.limit() as usize {
            return true;
        }

        self.displace_oldest_reservation()
    }

    fn expire_reservations(&self, grace: Duration) {
        self.reservations
            .retain(|_, left_at| left_at.elapsed() < grace);
    }

    // Gives up the reservation held longest, and reports whether there was one. A held slot
    // yields to real demand rather than outlasting it.
    fn displace_oldest_reservation(&self) -> bool {
        // The scan is scoped so every map guard is released before the removal below.
        let oldest = {
            let mut best: Option<(Arc<str>, Instant)> = None;
            for entry in self.reservations.iter() {
                let candidate = (entry.key().clone(), *entry.value());
                best = match &best {
                    Some((_, held_since)) if *held_since <= candidate.1 => best,
                    _ => Some(candidate),
                };
            }
            best.map(|(identity, _)| identity)
        };

        match oldest {
            Some(identity) => {
                self.reservations.remove(&identity);
                true
            }
            None => false,
        }
    }

    fn insert_connection(
        &self,
        device: u64,
        identity: Arc<str>,
        fingerprint: String,
        tx: mpsc::Sender<RoutedPacket>,
    ) {
        tracing::info!(
            "Registering connection for player: {} (connections: {})",
            identity,
            self.connections.len() + 1
        );
        self.name_index.insert(identity.clone(), device);
        if !fingerprint.is_empty() {
            self.fingerprint_index.insert(fingerprint.clone(), device);
        }
        let registered_name = identity.clone();
        let name_hash = InteractionTracker::hash_name(&identity);
        let replaced = self.connections.insert(
            device,
            ConnectionEntry {
                identity,
                sequence: ConnectionSequence::new_shared(),
                name_hash,
                fingerprint,
                tx,
                connected_at: Instant::now(),
            },
        );
        if let Some(metrics) = self.metrics.get() {
            // A connection id is unique for the lifetime of the process, so this only
            // replaces an entry the reaper has not reached yet; close out that session
            // first so connect/disconnect counters and session durations stay balanced.
            if let Some(old) = replaced {
                metrics.record_disconnect(
                    &old.identity,
                    old.connected_at.elapsed(),
                    WebSocketDeviceId::transport_of(device),
                );
            }
            metrics.record_connect(&registered_name, WebSocketDeviceId::transport_of(device));
        }
        self.push_gauges();
    }

    pub fn unregister(&self, device: u64) {
        if let Some((_, entry)) = self.connections.remove(&device) {
            // Only clear the index if it still points at THIS connection — a
            // reconnect that already reused the name must not be evicted here.
            let is_current = self
                .name_index
                .get(&entry.identity)
                .map(|v| *v == device)
                .unwrap_or(false);
            self.name_index.remove_if(&entry.identity, |_, v| *v == device);
            self.fingerprint_index
                .remove_if(&entry.fingerprint, |_, v| *v == device);
            // Guarded for the same reason, and it was not. A close arriving after the player
            // has already reconnected dropped the live connection's channel membership, and
            // their audio silently reverted to proximity until they rejoined the channel.
            if is_current {
                self.player_channel.remove(&entry.identity);
                // Guarded by `is_current` for the same reason the membership removal is: a
                // late close for a superseded connection would reserve a slot the player is
                // actively using, counting one live player against the limit twice.
                if self
                    .capacity
                    .get()
                    .is_some_and(|policy| !policy.is_unlimited())
                {
                    self.reservations
                        .insert(entry.identity.clone(), Instant::now());
                }
            }
            if let Some(metrics) = self.metrics.get() {
                metrics.record_disconnect(
                    &entry.identity,
                    entry.connected_at.elapsed(),
                    WebSocketDeviceId::transport_of(device),
                );
            }
            tracing::info!(
                "Unregistered connection for player: {} (connections: {})",
                entry.identity,
                self.connections.len()
            );
            self.push_gauges();
        }
    }

    pub fn broadcast_to_all(&self, packet: QuicNetworkPacket) {
        // Serialized once here and patched per recipient below. The encode doubles as the size
        // check, and the check has to happen before anything is stamped: an oversized packet
        // rejected inside the loop would consume a sequence number on every connection and read at
        // the receiver as loss. The sequence is fixed-width, so the length this proves is the
        // length every recipient's copy has.
        let mut probe = packet.clone();
        probe.stamp(u32::MAX);
        let template = match probe.to_datagram() {
            Ok(bytes) => bytes,
            Err(e) => {
                // A player list that does not fit is still deliverable in
                // pieces; every consumer merges players by name, so splitting
                // changes nothing observable. Dropping it whole would strand
                // every client's positions until the roster shrank.
                if let Some(halves) = Self::split_oversized(&packet) {
                    for half in halves {
                        self.broadcast_to_all(half);
                    }
                    return;
                }

                if packet.packet_type == PacketType::PlayerData {
                    if let Some(metrics) = self.metrics.get() {
                        metrics.record_position_oversize_drop();
                    }
                }

                if let Some(suppressed) = self.oversized_broadcast_log.should_log() {
                    tracing::error!(
                        suppressed,
                        packet_type = ?packet.packet_type,
                        "Failed to serialize broadcast: {}",
                        e
                    );
                }
                return;
            }
        };

        let mut dead_keys: Vec<u64> = Vec::new();

        for entry in self.connections.iter() {
            let Some(bytes) = entry.value().sequence.patch(&template) else {
                continue;
            };

            match entry.value().tx.try_send(RoutedPacket::Serialized(bytes)) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    tracing::debug!(
                        "Dropping broadcast packet for player {} (channel full)",
                        entry.value().identity,
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    dead_keys.push(*entry.key());
                }
            }
        }

        for key in dead_keys {
            self.unregister(key);
        }
    }

    // Halves an oversized player list. Returns `None` for anything that cannot
    // be divided -- a different packet type, or a single player already too
    // large -- which is what terminates the recursion in `broadcast_to_all`.
    fn split_oversized(packet: &QuicNetworkPacket) -> Option<[QuicNetworkPacket; 2]> {
        let QuicNetworkPacketData::PlayerData(data) = &packet.data else {
            return None;
        };

        if data.players.len() < 2 {
            return None;
        }

        let (head, tail) = data.players.split_at(data.players.len() / 2);

        let rebuild = |players: &[PlayerEnum]| QuicNetworkPacket {
            packet_type: packet.packet_type.clone(),
            sender: packet.sender.clone(),
            data: QuicNetworkPacketData::PlayerData(PlayerDataPacket::new(players.to_vec())),
            // Each half is re-entered through `broadcast_to_all`, which stamps per recipient.
            ..Default::default()
        };

        Some([rebuild(head), rebuild(tail)])
    }

    /// Delivers each player's position to that player alone.
    ///
    /// A client reads exactly one entry out of a position packet -- its own,
    /// which anchors the listener for spatial audio. Every other entry is
    /// decoded and discarded, because an emitter's position travels on the
    /// audio frame itself. Broadcasting the roster therefore sent N records to
    /// M clients to deliver M useful ones; addressing each player directly
    /// makes the cost O(connected clients) instead of O(roster x clients), and
    /// keeps a position packet at one player regardless of how large the realm
    /// grows.
    ///
    /// Returns how many clients were served, for metrics and tests.
    pub fn send_positions_to_owners(&self, packet: &QuicNetworkPacket) -> usize {
        let QuicNetworkPacketData::PlayerData(data) = &packet.data else {
            return 0;
        };

        let mut delivered = 0;

        for player in &data.players {
            let identity = player.identity().to_string();

            // Most of the roster is not on voice at all; skipping the
            // non-connected majority is the entire saving.
            if !self.name_index.contains_key(identity.as_str()) {
                continue;
            }

            let own = QuicNetworkPacket {
                packet_type: packet.packet_type.clone(),
                sender: packet.sender.clone(),
                data: QuicNetworkPacketData::PlayerData(PlayerDataPacket::new(vec![
                    player.clone(),
                ])),
                // `send_to_player` stamps this with that connection's sequence.
                ..Default::default()
            };

            if self.send_to_player(&identity, &own) {
                delivered += 1;
            }
        }

        delivered
    }

    /// Who currently holds a voice connection, by canonical identity.
    ///
    /// The position cache the mod feeds carries the whole world, most of which is not on
    /// voice at all — the same asymmetry `send_positions_to_owners` exploits. This is what
    /// lets the position feed report "in range, and nothing you say reaches them" rather
    /// than omitting those players and making them indistinguishable from nobody.
    // One index lookup, for the peer boundary to ask per packet whether a name a
    // peer used belongs to a player this server already serves.
    /// Closes the session opened with this certificate, telling the client why first.
    ///
    /// Returns whether a live session was found. The message is sent before unregistering so
    /// the client shows a reason rather than a bare disconnect; a client too old to decode it
    /// is dropped anyway, which is a worse message and not a missed revocation.
    pub fn revoke_session(&self, fingerprint: &str, reason: &str) -> bool {
        let Some(device) = self.device_for_fingerprint(fingerprint) else {
            return false;
        };

        let packet = QuicNetworkPacket {
            packet_type: PacketType::ServerError,
            data: QuicNetworkPacketData::ServerError(ServerErrorPacket {
                error_type: ServerErrorType::CertificateRevoked {
                    reason: reason.to_string(),
                },
                message: reason.to_string(),
            }),
            ..Default::default()
        };

        let identity = self
            .connections
            .get(&device)
            .map(|entry| entry.value().identity.clone());
        if let Some(identity) = identity {
            self.send_to_player(&identity, &packet);
        }

        self.unregister(device);
        true
    }

    /// The connection opened with this certificate, if it is still live.
    ///
    /// Keyed on the credential rather than the identity so revoking one certificate closes
    /// only the session it opened.
    pub fn device_for_fingerprint(&self, fingerprint: &str) -> Option<u64> {
        self.fingerprint_index.get(fingerprint).map(|e| *e.value())
    }

    pub fn has_live_client(&self, identity: &str) -> bool {
        self.name_index.contains_key(identity)
    }

    pub fn on_voice_identities(&self) -> std::collections::HashSet<String> {
        self.name_index
            .iter()
            .map(|entry| entry.key().to_string())
            .collect()
    }

    // Delivers a single packet to one connected player (by canonical identity) via the
    // O(1) name index. Returns whether a live connection received it; a closed
    // sender is reaped. Used by the control plane to route a ClientBound action to
    // its authenticated actor.
    pub fn send_to_player(&self, identity: &str, packet: &QuicNetworkPacket) -> bool {
        let device = match self.name_index.get(identity) {
            Some(id) => *id.value(),
            None => return false,
        };
        // Clone the sender and drop the DashMap ref before any send/unregister to
        // avoid holding a shard lock across a potential `unregister`.
        let (tx, sequence) = match self.connections.get(&device) {
            Some(entry) => (entry.value().tx.clone(), entry.value().sequence.clone()),
            None => return false,
        };
        let mut outbound = packet.clone();
        let bytes = match sequence.stamp(&mut outbound) {
            Some(b) => b,
            None => {
                // `stamp` logs the serialization failure itself; this keeps the counter so an
                // oversized position packet stays visible in metrics rather than only in logs.
                if packet.packet_type == PacketType::PlayerData {
                    if let Some(metrics) = self.metrics.get() {
                        metrics.record_position_oversize_drop();
                    }
                }
                return false;
            }
        };

        // Recorded here because the encoded bytes already exist; sizing the
        // packet again at the call site would double the work on a path that
        // runs for every player on every position tick.
        if packet.packet_type == PacketType::PlayerData {
            if let Some(metrics) = self.metrics.get() {
                let players = match &packet.data {
                    QuicNetworkPacketData::PlayerData(data) => data.players.len(),
                    _ => 0,
                };
                metrics.record_position_datagram(bytes.len(), players);
            }
        }

        match tx.try_send(RoutedPacket::Serialized(bytes)) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => false,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.unregister(device);
                false
            }
        }
    }

    pub fn update_player_channel(&self, identity: &str, channel_id: &str) {
        self.player_channel
            .insert(Arc::from(identity), Arc::from(channel_id));
        self.push_gauges();
    }

    pub fn remove_player_channel(&self, identity: &str) {
        self.player_channel.remove(identity);
        self.push_gauges();
    }

    pub fn remove_channel(&self, channel_id: &str) {
        self.player_channel.retain(|_, v| v.as_ref() != channel_id);
        self.push_gauges();
    }

    // The precomputed reach hash for a locally-connected player. `None` means the
    // name belongs to no live local connection.
    //
    // That covers two different populations. Server-injected audio (jukebox, webhook)
    // carries a synthetic sender and genuinely is not a player, which is the exclusion
    // this measurement wants. Relayed peers are real humans who happen to be
    // registered on their own server, and they are excluded too — so a deployment
    // whose conversation happens across the relay link reports near-zero reach, and
    // no emitted field distinguishes that from a quiet server.
    fn connection_name_hash(&self, identity: &str) -> Option<u64> {
        let device = *self.name_index.get(identity)?.value();
        self.connections
            .get(&device)
            .map(|entry| entry.value().name_hash)
    }

    /// `speaker` is the sender's player, resolved by the caller.
    ///
    /// Supplied rather than read off the frame, because the frame carries only a position and
    /// proximity needs a whole player — dimension, world and spectator. `player_cache` is still
    /// needed here, for the recipients.
    pub async fn route_audio_frame(
        &self,
        packet: &QuicNetworkPacket,
        speaker: Option<&PlayerEnum>,
        player_cache: &Arc<Cache<String, PlayerEnum>>,
        broadcast_range: f32,
        deafen_distance: f32,
    ) {
        let route_started = Instant::now();

        // The routing key the server stamped at ingress: a player's canonical identity, or the
        // service name for audio this server injected. An unstamped packet has no sender to
        // route from and a reduced one is never inbound, so both are unroutable.
        let Some(sender_identity) = packet.sender_key() else {
            return;
        };
        let sender_identity = sender_identity.as_str();

        let audio_frame = match &packet.data {
            QuicNetworkPacketData::AudioFrame(af) => af,
            _ => return,
        };

        let sender_channel: Option<Arc<str>> =
            self.player_channel.get(sender_identity).map(|r| r.clone());

        let original_spatial = audio_frame.spatial;
        let has_sender = speaker.is_some();

        // The speaker's PlayerEnum rides a heartbeat rather than every frame; recipients
        // reconstruct position from the last attached state. Per-speaker rather than
        // per-recipient, so the one-encode-per-variant template sharing below holds.
        let attach_sender = self.sender_attach_due(sender_identity, Instant::now());

        tracing::debug!(
            "route_audio_frame: sender={} original_spatial={:?} has_sender={} sender_channel={:?}",
            sender_identity,
            original_spatial,
            has_sender,
            sender_channel,
        );

        // Two envelope variants. Nothing differs between one recipient's datagram and another's
        // except the sequence number, and that occupies a fixed byte range, so each variant is
        // encoded at most once per frame and copied-and-patched per recipient.
        //
        // Built on first use rather than up front: a frame that only ever routes through a channel
        // never encodes the spatial variant, and vice versa.
        //
        // Stamped with zero purely to make the sequence range present in the encoding —
        // `ConnectionSequence::patch` overwrites it, and a packet the router rejects for proximity
        // or membership still consumes no number, because the counter is only touched there.
        let serialize_variant = |spatial: bool| -> Option<Vec<u8>> {
            let mut envelope = packet.clone();
            envelope.stamp(0);
            if let QuicNetworkPacketData::AudioFrame(ref mut af) = envelope.data {
                af.spatial = Some(spatial);
                if !attach_sender {
                    af.speaker = None;
                }
            }

            // Between heartbeats the recipient already knows which player this device is, so
            // the identity is left out and the device id stands in for it. A sender with no
            // device — injected audio — keeps its full form, because there is nothing a
            // recipient could resolve it from.
            if !attach_sender {
                if let Some(device) = envelope.sender.as_ref().and_then(|s| s.device()) {
                    envelope.sender = Some(PacketSender::Device(device));
                }
            }

            match envelope.to_datagram() {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    tracing::error!("failed to serialize audio envelope: {}", e);
                    None
                }
            }
        };

        let mut template_spatial: Option<Vec<u8>> = None;
        let mut template_channel: Option<Vec<u8>> = None;

        // Snapshot connections to release DashMap shard locks before any .await
        let snapshot: Vec<(
            u64,
            Arc<str>,
            u64,
            mpsc::Sender<RoutedPacket>,
            Arc<ConnectionSequence>,
        )> = self
            .connections
            .iter()
            .map(|entry| {
                (
                    *entry.key(),
                    entry.value().identity.clone(),
                    entry.value().name_hash,
                    entry.value().tx.clone(),
                    entry.value().sequence.clone(),
                )
            })
            .collect();

        let mut dead_keys: Vec<u64> = Vec::new();
        // Reach measures humans reaching humans, so only a sender with a live local
        // connection qualifies — otherwise a server whose players never speak to each
        // other would report healthy reach purely from background music. See
        // `connection_name_hash` for why this also drops relayed peers, who are not
        // background music.
        let sender_hash = self.connection_name_hash(sender_identity);

        for (device, recipient_identity, recipient_hash, tx, sequence) in &snapshot {
            if recipient_identity.as_ref() == sender_identity {
                continue;
            }

            let recipient_channel: Option<Arc<str>> = self
                .player_channel
                .get(recipient_identity.as_ref())
                .map(|r| r.clone());

            // Channel membership is cross-game by design: a channel id is shared
            // across games, so two members carrying different game prefixes in the
            // same channel hear each other. Only the fallback proximity path below
            // is gated to same-game (different games have unrelated coordinate
            // spaces, so spatial routing between them is meaningless).
            let in_same_channel = match (&sender_channel, &recipient_channel) {
                (Some(sc), Some(rc)) => sc == rc,
                _ => false,
            };

            // Which variant this recipient gets, and which route the delivery took. Selection
            // only — the stamp and the serialization happen after this block, so every
            // `continue` above leaves the sequence untouched.
            let (use_channel_variant, route) = if in_same_channel {
                tracing::debug!(
                    "route_audio_frame: {} -> {} IN_CHANNEL spatial={:?}",
                    sender_identity,
                    recipient_identity,
                    original_spatial,
                );
                // Same-channel members always receive the non-spatial variant so
                // the client skips distance-based volume attenuation. Without this,
                // a spatial=true packet would be zeroed by calculate_spatial_audio_data
                // when members are far apart, defeating the channel-bypass entirely.
                (true, InteractionRoute::Channel)
            } else {
                // Proximity is the only branch that needs coordinates, so both sides
                // must have reported a position to be compared at all. The recipient's
                // position is fetched here rather than above the branch because a
                // channel delivery never reads it, and the fetch is an awaited cache
                // lookup per recipient per frame.
                let Some(sp) = speaker else {
                    continue;
                };
                let rp = match player_cache.get(recipient_identity.as_ref()).await {
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

                if let Err(e) = sp.can_communicate_with(&rp, effective_range) {
                    tracing::debug!(
                        "Audio packet {} -> {} rejected: {}",
                        sender_identity,
                        recipient_identity,
                        e
                    );
                    continue;
                }

                // Some(false) is rejected outside channels
                match original_spatial {
                    Some(false) => continue,
                    Some(true) | None => (false, InteractionRoute::Proximity),
                }
            };

            let template: &[u8] = if use_channel_variant {
                if template_channel.is_none() {
                    match serialize_variant(false) {
                        Some(bytes) => template_channel = Some(bytes),
                        None => continue,
                    }
                }
                template_channel.as_deref().unwrap_or_default()
            } else {
                if template_spatial.is_none() {
                    match serialize_variant(true) {
                        Some(bytes) => template_spatial = Some(bytes),
                        None => continue,
                    }
                }
                template_spatial.as_deref().unwrap_or_default()
            };

            let Some(bytes_to_send) = sequence.patch(template) else {
                continue;
            };

            match tx.try_send(RoutedPacket::Serialized(bytes_to_send)) {
                Ok(()) => {
                    if let (Some(m), Some(sender_hash)) = (self.metrics.get(), sender_hash) {
                        m.record_interaction(route, sender_hash, *recipient_hash);
                    }
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    if let Some(m) = self.metrics.get() {
                        m.record_audio_route_drop();
                    }
                    tracing::debug!(
                        "Dropping audio packet for player {} (channel full)",
                        recipient_identity,
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    dead_keys.push(*device);
                }
            }
        }

        for key in dead_keys {
            self.unregister(key);
        }

        if let Some(m) = self.metrics.get() {
            m.record_audio_route(route_started.elapsed());
        }
    }
}
