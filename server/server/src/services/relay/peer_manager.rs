use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::{mpsc, oneshot};

use common::structs::relay::{AudioQuery, RelayEndpoint};

use common::structs::packet::PeerPresenceInjectPacket;

use super::audio_peer_query::{AudioPeerQuery, ResolvedAudio};
use super::audio_source::AudioSource;
use super::ingest_sink::RelayIngestSink;
use super::peer_link::{PeerDirection, PeerLink};
use super::peer_role::Caps;
use super::peer_table::PeerTable;
use super::presence::{PresenceProver, CHALLENGE_TTL};
use super::presence_gate::PresenceGate;
use super::relayed_packet::{PacketOrigin, RelayedPacket};

// Coordinates server↔server peer links: deterministic dial/accept tiebreak,
// one connection per peer endpoint (deduped across worlds), inbound packet
// injection (bypassing registration), outbound single-hop fan-out, and idle
// teardown. All peer I/O is off the audio hot path.
//
// Endpoint identity is the `host:port` string (advertised HTTPS endpoint). The s2n-quic dial/accept
// transport hookup lives in the runtime layer; this manager owns the routing
// decisions and link lifecycle that the transport drives.
pub struct PeerManager {
    self_endpoint: String,
    peer_table: Arc<PeerTable>,
    ingest: Arc<dyn RelayIngestSink>,
    presence: Arc<dyn PresenceGate>,
    // Concrete presence-proof orchestrator (challenge generation + echo
    // bookkeeping). Optional so the existing dial/accept/forward tests can keep
    // injecting an `AlwaysProven`/`NeverProven` gate without a real prover. The
    // runtime sets this to the same `PresenceProver` instance used as
    // the `presence` gate so the gate's `is_proven` and the orchestration agree.
    prover: Mutex<Option<Arc<PresenceProver>>>,
    // Cross-server jukebox responder. Optional so the dial/accept/forward and
    // presence tests need not construct a database-backed file lookup; the
    // runtime installs the concrete `AudioSource` via `set_audio_source`. When
    // present, an inbound `AudioQuery` is answered with `AudioAvailable` back
    // over the same peer link.
    audio_source: Mutex<Option<Arc<AudioSource>>>,
    // Outstanding cross-server jukebox discovery queries this server initiated,
    // keyed by `correlation_id`. `query_audio` broadcasts an `AudioQuery` and
    // parks a sender here; the first inbound `AudioAvailable` carrying that
    // correlation id resolves it and removes the entry (first responder wins;
    // later/unknown replies are dropped). Keying by correlation id rather than
    // `audio_id` lets two concurrent plays of the same file resolve independently.
    pending_audio_queries: Mutex<HashMap<String, oneshot::Sender<ResolvedAudio>>>,
    // endpoint string -> link
    links: Mutex<HashMap<String, PeerLink>>,
}

impl PeerManager {
    pub fn new(
        self_endpoint: RelayEndpoint,
        peer_table: Arc<PeerTable>,
        ingest: Arc<dyn RelayIngestSink>,
        presence: Arc<dyn PresenceGate>,
    ) -> Self {
        Self {
            self_endpoint: Self::endpoint_key(&self_endpoint),
            peer_table,
            ingest,
            presence,
            prover: Mutex::new(None),
            audio_source: Mutex::new(None),
            pending_audio_queries: Mutex::new(HashMap::new()),
            links: Mutex::new(HashMap::new()),
        }
    }

    pub fn new_shared(
        self_endpoint: RelayEndpoint,
        peer_table: Arc<PeerTable>,
        ingest: Arc<dyn RelayIngestSink>,
        presence: Arc<dyn PresenceGate>,
    ) -> Arc<Self> {
        Arc::new(Self::new(self_endpoint, peer_table, ingest, presence))
    }

    // Canonical `host:port` identity (advertised HTTPS endpoint) for an endpoint.
    pub fn endpoint_key(ep: &RelayEndpoint) -> String {
        format!("{}:{}", ep.host, ep.port)
    }

    // Installs the concrete presence-proof orchestrator. Called by the runtime
    // with the same `PresenceProver` that backs the `presence` gate.
    pub fn set_prover(&self, prover: Arc<PresenceProver>) {
        *self.prover.lock().expect("prover poisoned") = Some(prover);
    }

    fn prover(&self) -> Option<Arc<PresenceProver>> {
        self.prover.lock().expect("prover poisoned").clone()
    }

    // Installs the cross-server jukebox responder. Called by the runtime with an
    // `AudioSource` wired to the local audio file lookup + stream-token cache.
    pub fn set_audio_source(&self, source: Arc<AudioSource>) {
        *self.audio_source.lock().expect("audio source poisoned") = Some(source);
    }

    fn audio_source(&self) -> Option<Arc<AudioSource>> {
        self.audio_source.lock().expect("audio source poisoned").clone()
    }

    // Generates a fresh presence challenge for every active world that has a
    // non-self peer not yet proven, returning the `(hashed_world, packet)` pairs
    // the runtime must deliver as `PeerPresenceInject` to this server's OWN local
    // client(s) in that world (NEVER to a peer — see `PresenceProver` invariant).
    // Empty when no prover is installed.
    pub fn challenges_to_send(&self, now: Instant) -> Vec<(String, PeerPresenceInjectPacket)> {
        let prover = match self.prover() {
            Some(p) => p,
            None => return Vec::new(),
        };
        let ttl_ms = CHALLENGE_TTL.as_millis() as u32;
        let mut out = Vec::new();
        for world in self.peer_table.active_worlds() {
            let needs_challenge = self.peer_table.peers_for_world(&world).iter().any(|peer| {
                let key = Self::endpoint_key(peer);
                // A challenge is needed until the peer has echoed a token WE
                // injected for this world. The OUTBOUND half (us echoing their
                // token) is driven by their own challenge, so it must not
                // suppress ours.
                !Self::is_self(&self.self_endpoint, &key) && !prover.peer_proved_us(peer, &world)
            });
            if needs_challenge {
                let token = prover.new_challenge(&world, now);
                out.push((world, PeerPresenceInjectPacket { token, ttl_ms }));
            }
        }
        out
    }

    // Routes a `PeerPresenceObserved` token a peer echoed back to us over the
    // peer link into the prover, marking the peer proven for the world it echoed
    // a token for (no-op without a prover or on an unknown/expired token).
    pub fn route_observed_from_peer(&self, peer_ep: &str, token: &str, now: Instant) {
        if let Some(prover) = self.prover() {
            prover.record_observed_from_peer(peer_ep, token, now);
        }
    }

    // Records that one of THIS server's own local clients observed a token in the
    // realm (the peer challenged us). The token is echoed back to the peer
    // link(s) via `tokens_to_echo_to_peer` only when it matches a challenge we
    // are participating in (enforced inside the prover); arbitrary
    // strings are dropped.
    pub fn on_local_client_observed(&self, token: &str, now: Instant) {
        if let Some(prover) = self.prover() {
            prover.on_client_observed(token, now);
        }
    }

    // Registers a token a peer is expected to challenge us with for
    // `hashed_world`, so our client's later observation of it is treated as a
    // known (echo-eligible) token rather than dropped.
    pub fn expect_observed(&self, token: &str, hashed_world: &str, now: Instant) {
        if let Some(prover) = self.prover() {
            prover.expect_observed(token, hashed_world, now);
        }
    }

    // Drains the `(token, hashed_world)` pairs our local clients observed that
    // must be echoed to peers over the link as `PeerPresenceObserved`. Empty
    // without a prover. World-attributed so the echo records the mutual-proof
    // half for the correct world.
    pub fn tokens_to_echo_to_peer(&self) -> Vec<(String, String)> {
        match self.prover() {
            Some(prover) => prover.tokens_to_echo_to_peer(),
            None => Vec::new(),
        }
    }

    // Deterministic tiebreak: the lexically-lower endpoint dials; the other
    // waits to accept. False when equal (that is self — never relay to self).
    pub fn should_initiate(self_ep: &str, peer_ep: &str) -> bool {
        self_ep < peer_ep
    }

    // True when the candidate endpoint is this server itself.
    pub fn is_self(self_ep: &str, peer_ep: &str) -> bool {
        self_ep == peer_ep
    }

    // Single-hop loop prevention: a packet that arrived from a peer is never
    // forwarded onward. Local-origin packets are forwardable only when peers
    // exist for their world.
    pub fn should_forward_to_peer(relayed: &RelayedPacket, has_peers: bool) -> bool {
        match relayed.origin {
            PacketOrigin::FromPeer => false,
            PacketOrigin::Local => has_peers,
        }
    }

    // Advisory hub preference. More cores wins;
    // tie broken by more open-peer headroom. `Greater` means "I (self_caps)
    // should bear the hub role". Never consulted by v1 routing.
    pub fn prefer_hub(self_caps: &Caps, peer_caps: &Caps) -> Ordering {
        match self_caps.cores.cmp(&peer_caps.cores) {
            Ordering::Equal => self_caps.open_peers.cmp(&peer_caps.open_peers),
            other => other,
        }
    }

    // Reconciles the link table against the current `PeerTable` discovery view.
    // For each non-self peer this server should initiate to (and that presence
    // has proven for the world), ensures a `Dialing` link exists. Acceptor-side
    // links are created lazily when an inbound connection arrives. Returns the
    // endpoints this server intends to dial.
    pub fn reconcile(&self, now: Instant) -> Vec<String> {
        let mut to_dial = Vec::new();
        let worlds = self.peer_table.active_worlds();
        let mut links = self.links.lock().expect("peer link map poisoned");

        for world in &worlds {
            for peer in self.peer_table.peers_for_world(world) {
                let key = Self::endpoint_key(&peer);
                if Self::is_self(&self.self_endpoint, &key) {
                    continue;
                }
                if !self.presence.is_proven(&peer, world) {
                    continue;
                }
                if !Self::should_initiate(&self.self_endpoint, &key) {
                    continue;
                }
                if !links.contains_key(&key) {
                    links.insert(
                        key.clone(),
                        PeerLink::new(&key, PeerDirection::Initiator, now),
                    );
                    to_dial.push(key);
                }
            }
        }

        to_dial
    }

    // The first active world that lists `peer_ep` as a peer. Used by
    // the orchestrator to scope a dial intent's peer-cert fetch to a shared world.
    pub fn world_for_peer(&self, peer_ep: &str) -> Option<String> {
        for world in self.peer_table.active_worlds() {
            let in_world = self
                .peer_table
                .peers_for_world(&world)
                .iter()
                .any(|p| Self::endpoint_key(p) == peer_ep);
            if in_world {
                return Some(world);
            }
        }
        None
    }

    // Registers an inbound peer connection. If a dial to the same endpoint was
    // pending it is adopted/cancelled; otherwise a fresh acceptor link is
    // created. Returns true when an existing pending dial was cancelled.
    pub fn register_inbound(&self, peer_ep: &str, now: Instant) -> bool {
        let mut links = self.links.lock().expect("peer link map poisoned");
        match links.get_mut(peer_ep) {
            Some(link) => link.adopt_inbound(now),
            None => {
                links.insert(
                    peer_ep.to_string(),
                    PeerLink::new(peer_ep, PeerDirection::Acceptor, now),
                );
                false
            }
        }
    }

    // Hands the receive half of a link's bounded outbound queue to the peer-writer
    // task. The dialer's write pump drains this receiver onto the
    // QUIC connection; without this call the queue `forward_local` fills was never
    // drained (filled to 1024 then silently dropped). Takeable once per link.
    pub fn take_outbound_receiver(
        &self,
        peer_ep: &str,
    ) -> Option<mpsc::Receiver<RelayedPacket>> {
        let mut links = self.links.lock().expect("peer link map poisoned");
        links.get_mut(peer_ep).and_then(|link| link.take_outbound_receiver())
    }

    pub fn link_count(&self) -> usize {
        self.links.lock().expect("peer link map poisoned").len()
    }

    pub fn has_link(&self, peer_ep: &str) -> bool {
        self.links.lock().expect("peer link map poisoned").contains_key(peer_ep)
    }

    // Inbound relayed packet: FAIL CLOSED on presence proof. Before
    // a relayed AUDIO/position packet is published into the local broadcast path,
    // the originating peer MUST be mutually presence-proven for that packet's
    // `relay_world_uuid` . Presence-proof CONTROL
    // packets (`PeerPresenceObserved`/`PeerPresenceInject` echoes over the peer
    // link) are exempt: they must be processed pre-proof so the handshake can
    // complete. Everything else from an unproven peer — or any packet carrying no
    // resolvable relay world — is DROPPED.
    //
    // On success it marks the originating link active then publishes into the
    // SAME broadcast path local clients use, BYPASSING registration. Positions
    // ride into `player_cache` via that path (ephemeral, for proximity) — no
    // `player` record is ever created.
    pub async fn ingest(&self, peer_ep: &str, packet: common::structs::packet::QuicNetworkPacket) {
        if !self.is_ingest_authorized(peer_ep, &packet) {
            tracing::warn!(
                "dropping relayed {:?} from unproven/unscoped peer {} (presence gate, fail-closed)",
                packet.packet_type,
                peer_ep
            );
            return;
        }
        {
            let mut links = self.links.lock().expect("peer link map poisoned");
            if let Some(link) = links.get_mut(peer_ep) {
                link.mark_activity(Instant::now());
            }
        }

        // Cross-server jukebox discovery rides the peer link but is not local
        // broadcast traffic: an `AudioQuery` is answered with `AudioAvailable`
        // back over the same link and never published.
        use common::structs::packet::{PacketType, QuicNetworkPacketData};
        if packet.packet_type == PacketType::AudioQuery {
            if let QuicNetworkPacketData::AudioQuery(query) = &packet.data {
                self.answer_audio_query(peer_ep, query).await;
            }
            return;
        }

        if packet.packet_type == PacketType::AudioAvailable {
            if let QuicNetworkPacketData::AudioAvailable(available) = &packet.data {
                self.resolve_audio_available(peer_ep, available);
            }
            return;
        }

        self.ingest.publish(packet).await;
    }

    // Routes an inbound `AudioQuery` to the responder and, when this server holds
    // the file, enqueues the `AudioAvailable` reply back onto the originating
    // peer link. No-op when no responder is installed or the file is absent.
    async fn answer_audio_query(
        &self,
        peer_ep: &str,
        query: &common::structs::relay::AudioQuery,
    ) {
        use common::structs::packet::{PacketType, QuicNetworkPacket, QuicNetworkPacketData};
        let source = match self.audio_source() {
            Some(s) => s,
            None => return,
        };
        let available = match source.handle_query(query).await {
            Some(a) => a,
            None => return,
        };
        let reply = QuicNetworkPacket {
            packet_type: PacketType::AudioAvailable,
            owner: None,
            data: QuicNetworkPacketData::AudioAvailable(available),
        };
        self.enqueue_to_link(peer_ep, &RelayedPacket::local(reply));
    }

    // Fulfiller half of the discovery handshake: parks an entry keyed by
    // `correlation_id`, broadcasts an `AudioQuery` to every live peer link, and
    // returns the receiver the first matching `AudioAvailable` resolves. The
    // correlation id (the playback `event_id`) keeps concurrent queries for the
    // same `audio_id` from clobbering one another. No-op fan-out when there are no
    // peer links — the receiver then resolves only if a reply somehow arrives,
    // otherwise the caller times out.
    pub fn query_audio(&self, audio_id: &str, correlation_id: &str) -> oneshot::Receiver<ResolvedAudio> {
        use common::structs::packet::{PacketType, QuicNetworkPacket, QuicNetworkPacketData};
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self
                .pending_audio_queries
                .lock()
                .expect("pending audio queries poisoned");
            pending.insert(correlation_id.to_string(), tx);
        }
        let query = QuicNetworkPacket {
            packet_type: PacketType::AudioQuery,
            owner: None,
            data: QuicNetworkPacketData::AudioQuery(AudioQuery {
                audio_id: audio_id.to_string(),
                correlation_id: correlation_id.to_string(),
            }),
        };
        self.enqueue_to_all_links(&RelayedPacket::local(query));
        rx
    }

    // Resolves the outstanding query for an inbound `AudioAvailable`, pairing it
    // with the responder endpoint the reply rode in on. Matched by correlation
    // id. First responder wins: the entry is removed on the first match, so later
    // replies for the same correlation (or replies with no outstanding query) are
    // dropped.
    fn resolve_audio_available(
        &self,
        peer_ep: &str,
        available: &common::structs::relay::AudioAvailable,
    ) {
        let sender = {
            let mut pending = self
                .pending_audio_queries
                .lock()
                .expect("pending audio queries poisoned");
            pending.remove(&available.correlation_id)
        };
        if let Some(sender) = sender {
            let resolved = ResolvedAudio {
                available: available.clone(),
                responder: Self::endpoint_from_key(peer_ep),
            };
            let _ = sender.send(resolved);
        }
    }

    // Enqueues a packet onto a single peer link's outbound queue (drop-on-full,
    // never blocks). Used to send a discovery reply back to the peer that asked.
    fn enqueue_to_link(&self, peer_ep: &str, packet: &RelayedPacket) {
        let mut links = self.links.lock().expect("peer link map poisoned");
        if let Some(link) = links.get_mut(peer_ep) {
            if link.is_closed() {
                return;
            }
            if link.outbound_sender().try_send(packet.clone()).is_err() {
                tracing::debug!(
                    "dropping audio discovery reply for peer {} (queue full/closed)",
                    peer_ep
                );
            }
        }
    }

    // Decides whether an inbound relayed packet may be published.
    // Control packets used to COMPLETE the presence handshake are always allowed
    // (they carry no audio and gate nothing). Media/position packets are allowed
    // only when the originating peer is mutually presence-proven for the packet's
    // resolved `relay_world_uuid`; a packet with no resolvable relay world is
    // dropped (fail closed).
    fn is_ingest_authorized(
        &self,
        peer_ep: &str,
        packet: &common::structs::packet::QuicNetworkPacket,
    ) -> bool {
        use common::structs::packet::PacketType;
        match packet.packet_type {
            PacketType::PeerPresenceObserved | PacketType::PeerPresenceInject => true,
            // Peer-link discovery handshake for cross-server jukebox. These
            // carry no Minecraft sender, so they have no resolvable relay world;
            // they ride peer links that are already mutually presence-proven, so
            // they are allowed just like the presence-control packets above.
            PacketType::AudioQuery | PacketType::AudioAvailable => true,
            _ => match Self::packet_relay_world(packet) {
                Some(world) => {
                    let endpoint = Self::endpoint_from_key(peer_ep);
                    self.presence.is_proven(&endpoint, &world)
                }
                None => false,
            },
        }
    }

    // Extracts the `relay_world_uuid` a relayed packet is scoped to, from the
    // sender embedded in an audio frame or the player in a position packet.
    // `None` when the packet carries no Minecraft sender / relay world.
    fn packet_relay_world(
        packet: &common::structs::packet::QuicNetworkPacket,
    ) -> Option<String> {
        use common::structs::packet::QuicNetworkPacketData;
        let player = match &packet.data {
            QuicNetworkPacketData::AudioFrame(frame) => frame.sender.as_ref(),
            QuicNetworkPacketData::PlayerPosition(pos) => Some(&pos.player),
            _ => None,
        }?;
        player
            .as_minecraft()
            .and_then(|mc| mc.relay_world_uuid.clone())
    }

    // Parses a `host:port` endpoint key back into a `RelayEndpoint` for the
    // presence gate. A key that does not end in a numeric port yields port 0,
    // which the gate will never have proven — preserving fail-closed behavior.
    fn endpoint_from_key(key: &str) -> RelayEndpoint {
        match key.rsplit_once(':') {
            Some((host, port)) => RelayEndpoint {
                host: host.to_string(),
                port: port.parse().unwrap_or(0),
                primary: false,
            },
            None => RelayEndpoint {
                host: key.to_string(),
                port: 0,
                primary: false,
            },
        }
    }

    // Outbound fan-out for a local-origin packet. For each peer link whose world
    // has peers, enqueues one copy via `try_send` (drop-on-full; never blocks
    // the audio path). Relayed-origin packets are refused here (single hop).
    // Returns the number of peer queues the packet was enqueued onto.
    pub fn forward_local(&self, relayed: &RelayedPacket, hashed_world: &str) -> usize {
        if relayed.is_relayed() {
            return 0;
        }
        let peers = self.peer_table.peers_for_world(hashed_world);
        let has_peers = peers
            .iter()
            .any(|p| !Self::is_self(&self.self_endpoint, &Self::endpoint_key(p)));
        if !Self::should_forward_to_peer(relayed, has_peers) {
            return 0;
        }

        let mut sent = 0;
        let mut links = self.links.lock().expect("peer link map poisoned");
        let now = Instant::now();
        for peer in &peers {
            let key = Self::endpoint_key(peer);
            if Self::is_self(&self.self_endpoint, &key) {
                continue;
            }
            if let Some(link) = links.get_mut(&key) {
                if link.is_closed() {
                    continue;
                }
                match link.outbound_sender().try_send(relayed.clone()) {
                    Ok(()) => {
                        link.mark_activity(now);
                        sent += 1;
                    }
                    Err(_) => {
                        tracing::debug!("dropping relay packet for peer {} (queue full/closed)", key);
                    }
                }
            }
        }
        sent
    }

    // Enqueues a control/echo packet onto every live peer link's outbound queue
    // (used to fan a `PeerPresenceObserved` echo to peers). Returns the endpoint
    // keys it reached so the caller can record the mutual-proof half per peer.
    // Drop-on-full like the audio fan-out — never blocks.
    pub fn enqueue_to_all_links(&self, packet: &RelayedPacket) -> Vec<String> {
        let mut reached = Vec::new();
        let mut links = self.links.lock().expect("peer link map poisoned");
        for (key, link) in links.iter_mut() {
            if link.is_closed() {
                continue;
            }
            if link.outbound_sender().try_send(packet.clone()).is_ok() {
                reached.push(key.clone());
            }
        }
        reached
    }

    // Endpoint keys of every live (non-closed) peer link. Lets the echo path
    // record the mutual-proof half against the peers an echo is dispatched to.
    pub fn live_link_endpoints(&self) -> Vec<String> {
        self.links
            .lock()
            .expect("peer link map poisoned")
            .iter()
            .filter(|(_, link)| !link.is_closed())
            .map(|(key, _)| key.clone())
            .collect()
    }

    // Closes peer links idle for >= IDLE_TIMEOUT and returns their endpoints so
    // the transport layer can send a close frame (bilateral teardown — the peer
    // drops its side on receiving it). Re-establishment is lazy on the next
    // relay-worthy packet via `reconcile`.
    pub fn sweep_idle(&self, now: Instant) -> Vec<String> {
        let mut closed = Vec::new();
        let mut links = self.links.lock().expect("peer link map poisoned");
        for (key, link) in links.iter_mut() {
            if link.is_idle(now) && !link.is_closed() {
                link.close();
                closed.push(key.clone());
            }
        }
        links.retain(|_, link| !link.is_closed());
        closed
    }
}

impl AudioPeerQuery for PeerManager {
    fn query_audio(&self, audio_id: &str, correlation_id: &str) -> oneshot::Receiver<ResolvedAudio> {
        PeerManager::query_audio(self, audio_id, correlation_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::relay::presence_gate::{AlwaysProven, NeverProven};
    use common::structs::packet::{
        AudioFramePacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.into(),
            port,
            primary: false,
        }
    }

    // Spy sink that also asserts no registration ever happens (there is no
    // registrar on this path; the counter proves only `publish` is reached).
    struct SpySink {
        published: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl RelayIngestSink for SpySink {
        async fn publish(&self, _packet: QuicNetworkPacket) {
            self.published.fetch_add(1, AtomicOrdering::SeqCst);
        }
    }

    fn audio_packet() -> QuicNetworkPacket {
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            owner: None,
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![9, 9, 9],
                48000,
                None,
                Some(true),
            )),
        }
    }

    // An audio packet whose sender is a Minecraft player scoped to `relay_world`,
    // so the inbound presence gate can resolve a world to authorize against.
    fn audio_packet_in_world(relay_world: &str) -> QuicNetworkPacket {
        use common::game_data::Dimension;
        use common::players::MinecraftPlayer;
        use common::{Coordinate, Orientation, PlayerEnum};
        let sender = PlayerEnum::Minecraft(MinecraftPlayer {
            name: "alice".into(),
            coordinates: Coordinate { x: 0.0, y: 0.0, z: 0.0 },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: Some(relay_world.into()),
        });
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            owner: None,
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![9, 9, 9],
                48000,
                Some(sender),
                Some(true),
            )),
        }
    }

    fn manager_with(
        self_ep: RelayEndpoint,
        sink: Arc<dyn RelayIngestSink>,
        presence: Arc<dyn PresenceGate>,
    ) -> PeerManager {
        PeerManager::new(self_ep, PeerTable::new_shared(), sink, presence)
    }

    #[test]
    fn tiebreak_lower_endpoint_dials() {
        assert!(PeerManager::should_initiate("a:1", "b:1"));
        assert!(!PeerManager::should_initiate("b:1", "a:1"));
        assert!(!PeerManager::should_initiate("a:1", "a:1"));
    }

    #[test]
    fn is_self_when_equal() {
        assert!(PeerManager::is_self("a:1", "a:1"));
        assert!(!PeerManager::is_self("a:1", "a:2"));
    }

    #[test]
    fn never_rerelay_relayed_packets() {
        let p = RelayedPacket::from_peer(audio_packet());
        assert!(!PeerManager::should_forward_to_peer(&p, true));
    }

    #[test]
    fn forward_local_only_when_peers_exist() {
        let local = RelayedPacket::local(audio_packet());
        assert!(PeerManager::should_forward_to_peer(&local, true));
        assert!(!PeerManager::should_forward_to_peer(&local, false));
    }

    #[test]
    fn capacity_preference_prefers_more_headroom() {
        let mine = Caps {
            cores: 20,
            open_peers: 5,
        };
        let theirs = Caps {
            cores: 1,
            open_peers: 0,
        };
        assert_eq!(PeerManager::prefer_hub(&mine, &theirs), Ordering::Greater);
    }

    #[test]
    fn capacity_preference_breaks_core_tie_on_open_peers() {
        let mine = Caps {
            cores: 4,
            open_peers: 0,
        };
        let theirs = Caps {
            cores: 4,
            open_peers: 5,
        };
        assert_eq!(PeerManager::prefer_hub(&mine, &theirs), Ordering::Less);
    }

    #[tokio::test]
    async fn ingest_publishes_to_broadcast_without_registration() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        // AlwaysProven authorizes; the packet must carry a resolvable relay world.
        let mgr = manager_with(ep("self", 1), sink.clone(), Arc::new(AlwaysProven));
        mgr.register_inbound("peer:2", Instant::now());
        mgr.ingest("peer:2", audio_packet_in_world("W")).await;
        assert_eq!(sink.published.load(AtomicOrdering::SeqCst), 1);
    }

    // An inbound relayed AUDIO packet from a peer that is NOT yet
    // mutually presence-proven for the packet's relay world must be DROPPED
    // (fail closed) — never published to the broadcast sink. Once the peer is
    // mutually proven for that world, the same packet IS published.
    #[tokio::test]
    async fn ingest_drops_audio_from_unproven_peer_then_publishes_once_proven() {
        use crate::services::relay::presence::PresenceProver;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let prover = PresenceProver::new_shared();
        let peer = ep("peerX", 7000);
        let key = PeerManager::endpoint_key(&peer);
        let mgr = PeerManager::new(ep("self", 1), PeerTable::new_shared(), sink.clone(), prover.clone());
        mgr.set_prover(prover.clone());
        mgr.register_inbound(&key, Instant::now());

        // Un-proven: the relayed audio must be dropped.
        mgr.ingest(&key, audio_packet_in_world("W")).await;
        assert_eq!(
            sink.published.load(AtomicOrdering::SeqCst),
            0,
            "un-proven peer's relayed audio must not be published"
        );

        // Complete the mutual proof for world W against this peer.
        let now = Instant::now();
        let token = prover.new_challenge("W", now);
        prover.record_observed_from_peer(&key, &token, now);
        prover.record_echoed_to_peer(&key, "W");
        assert!(mgr.presence.is_proven(&peer, "W"));

        // Now the same relayed audio IS published.
        mgr.ingest(&key, audio_packet_in_world("W")).await;
        assert_eq!(
            sink.published.load(AtomicOrdering::SeqCst),
            1,
            "once mutually proven, the relayed audio is published"
        );
    }

    // Proof is world-scoped at ingest too — a peer proven for W must
    // NOT have its audio for a DIFFERENT world W2 published.
    #[tokio::test]
    async fn ingest_proof_does_not_cross_worlds() {
        use crate::services::relay::presence::PresenceProver;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let prover = PresenceProver::new_shared();
        let peer = ep("peerX", 7000);
        let key = PeerManager::endpoint_key(&peer);
        let mgr = PeerManager::new(ep("self", 1), PeerTable::new_shared(), sink.clone(), prover.clone());
        mgr.set_prover(prover.clone());
        mgr.register_inbound(&key, Instant::now());

        let now = Instant::now();
        let token = prover.new_challenge("W", now);
        prover.record_observed_from_peer(&key, &token, now);
        prover.record_echoed_to_peer(&key, "W");

        // proven for W, but the packet is scoped to W2 -> dropped.
        mgr.ingest(&key, audio_packet_in_world("W2")).await;
        assert_eq!(sink.published.load(AtomicOrdering::SeqCst), 0);
    }

    // Presence-proof CONTROL packets (the echoes that COMPLETE the
    // handshake over the peer link) must be processed pre-proof, otherwise the
    // mutual proof could never complete. They are published even from an
    // un-proven peer.
    #[tokio::test]
    async fn ingest_allows_presence_control_packets_pre_proof() {
        use common::structs::packet::PeerPresenceObservedPacket;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let mgr = manager_with(ep("self", 1), sink.clone(), Arc::new(NeverProven));
        let key = "peerX:7000";
        mgr.register_inbound(key, Instant::now());
        let control = QuicNetworkPacket {
            packet_type: PacketType::PeerPresenceObserved,
            owner: None,
            data: QuicNetworkPacketData::PeerPresenceObserved(PeerPresenceObservedPacket {
                token: "tok".into(),
            }),
        };
        mgr.ingest(key, control).await;
        assert_eq!(
            sink.published.load(AtomicOrdering::SeqCst),
            1,
            "presence control packets must flow pre-proof so the handshake can complete"
        );
    }

    // The cross-server jukebox discovery packets carry no Minecraft sender, so
    // they have no resolvable relay world. They must be allowed through the
    // ingest gate (they ride already-proven peer links), while a non-relay audio
    // packet from an unproven peer is still dropped fail-closed.
    #[test]
    fn ingest_allows_audio_query_but_not_unproven_audio() {
        use common::structs::relay::AudioQuery;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let mgr = manager_with(ep("self", 1), sink, Arc::new(NeverProven));
        let key = "peerX:7000";
        let query = QuicNetworkPacket {
            packet_type: PacketType::AudioQuery,
            owner: None,
            data: QuicNetworkPacketData::AudioQuery(AudioQuery {
                audio_id: "audio-1".into(),
                correlation_id: "corr-1".into(),
            }),
        };
        assert!(
            mgr.is_ingest_authorized(key, &query),
            "AudioQuery must pass the ingest gate on a peer link"
        );
        assert!(
            !mgr.is_ingest_authorized(key, &audio_packet()),
            "a non-relay audio packet from an unproven peer must still be dropped"
        );
    }

    // The fulfiller registers an outstanding query, then an inbound
    // `AudioAvailable` for that id resolves the receiver with the responder's
    // endpoint parsed from the peer link it arrived on.
    #[tokio::test]
    async fn query_audio_resolves_on_matching_available() {
        use common::structs::relay::AudioAvailable;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let mgr = manager_with(ep("self", 1), sink, Arc::new(AlwaysProven));
        let rx = mgr.query_audio("audio-1", "corr-1");
        let available = QuicNetworkPacket {
            packet_type: PacketType::AudioAvailable,
            owner: None,
            data: QuicNetworkPacketData::AudioAvailable(AudioAvailable {
                audio_id: "audio-1".into(),
                stream_token: "tok-1".into(),
                correlation_id: "corr-1".into(),
            }),
        };
        mgr.register_inbound("peerX:7000", Instant::now());
        mgr.ingest("peerX:7000", available).await;

        let resolved = rx.await.expect("receiver should resolve");
        assert_eq!(resolved.available.audio_id, "audio-1");
        assert_eq!(resolved.available.stream_token, "tok-1");
        assert_eq!(resolved.responder.host, "peerX");
        assert_eq!(resolved.responder.port, 7000);
    }

    // An `AudioAvailable` for an id with no outstanding query is a no-op: it
    // resolves nothing and must not panic.
    #[tokio::test]
    async fn audio_available_for_unknown_id_is_noop() {
        use common::structs::relay::AudioAvailable;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let mgr = manager_with(ep("self", 1), sink.clone(), Arc::new(AlwaysProven));
        let available = QuicNetworkPacket {
            packet_type: PacketType::AudioAvailable,
            owner: None,
            data: QuicNetworkPacketData::AudioAvailable(AudioAvailable {
                audio_id: "never-queried".into(),
                stream_token: "tok".into(),
                correlation_id: "corr-unknown".into(),
            }),
        };
        mgr.register_inbound("peerX:7000", Instant::now());
        mgr.ingest("peerX:7000", available).await;
        assert_eq!(
            sink.published.load(AtomicOrdering::SeqCst),
            0,
            "AudioAvailable is never published to the broadcast sink"
        );
    }

    // First responder wins: once an `AudioAvailable` resolves the query, a second
    // one for the same id finds no outstanding entry and is dropped.
    #[tokio::test]
    async fn first_audio_available_wins() {
        use common::structs::relay::AudioAvailable;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let mgr = manager_with(ep("self", 1), sink, Arc::new(AlwaysProven));
        let rx = mgr.query_audio("audio-1", "corr-1");
        mgr.register_inbound("peerA:7000", Instant::now());
        mgr.register_inbound("peerB:7000", Instant::now());

        let first = QuicNetworkPacket {
            packet_type: PacketType::AudioAvailable,
            owner: None,
            data: QuicNetworkPacketData::AudioAvailable(AudioAvailable {
                audio_id: "audio-1".into(),
                stream_token: "first".into(),
                correlation_id: "corr-1".into(),
            }),
        };
        let second = QuicNetworkPacket {
            packet_type: PacketType::AudioAvailable,
            owner: None,
            data: QuicNetworkPacketData::AudioAvailable(AudioAvailable {
                audio_id: "audio-1".into(),
                stream_token: "second".into(),
                correlation_id: "corr-1".into(),
            }),
        };
        mgr.ingest("peerA:7000", first).await;
        mgr.ingest("peerB:7000", second).await;

        let resolved = rx.await.expect("receiver should resolve");
        assert_eq!(resolved.available.stream_token, "first");
        assert_eq!(resolved.responder.host, "peerA");
    }

    // Two concurrent queries for the SAME `audio_id` but different correlation
    // ids resolve independently: each `AudioAvailable` carrying a correlation id
    // resolves only its matching receiver, with neither clobbering the other.
    #[tokio::test]
    async fn concurrent_same_audio_id_queries_resolve_independently() {
        use common::structs::relay::AudioAvailable;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let mgr = manager_with(ep("self", 1), sink, Arc::new(AlwaysProven));
        let rx_a = mgr.query_audio("audio-1", "corr-a");
        let rx_b = mgr.query_audio("audio-1", "corr-b");
        mgr.register_inbound("peerA:7000", Instant::now());
        mgr.register_inbound("peerB:7000", Instant::now());

        let reply_a = QuicNetworkPacket {
            packet_type: PacketType::AudioAvailable,
            owner: None,
            data: QuicNetworkPacketData::AudioAvailable(AudioAvailable {
                audio_id: "audio-1".into(),
                stream_token: "tok-a".into(),
                correlation_id: "corr-a".into(),
            }),
        };
        let reply_b = QuicNetworkPacket {
            packet_type: PacketType::AudioAvailable,
            owner: None,
            data: QuicNetworkPacketData::AudioAvailable(AudioAvailable {
                audio_id: "audio-1".into(),
                stream_token: "tok-b".into(),
                correlation_id: "corr-b".into(),
            }),
        };
        mgr.ingest("peerA:7000", reply_a).await;
        mgr.ingest("peerB:7000", reply_b).await;

        let resolved_a = rx_a.await.expect("query A should resolve");
        let resolved_b = rx_b.await.expect("query B should resolve");
        assert_eq!(resolved_a.available.stream_token, "tok-a");
        assert_eq!(resolved_a.responder.host, "peerA");
        assert_eq!(resolved_b.available.stream_token, "tok-b");
        assert_eq!(resolved_b.responder.host, "peerB");
    }

    #[test]
    fn reconcile_initiates_only_to_higher_proven_peers() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        // self is "a:1"; peers "b:1" (higher -> dial) and "0:1" (lower -> wait)
        table.set_world_peers("W", vec![ep("b", 1), ep("0", 1)]);
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(AlwaysProven));
        let dialed = mgr.reconcile(Instant::now());
        assert_eq!(dialed, vec!["b:1".to_string()]);
        assert!(mgr.has_link("b:1"));
        assert!(!mgr.has_link("0:1"));
    }

    #[test]
    fn reconcile_skips_unproven_peers() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("z", 1)]);
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(NeverProven));
        assert!(mgr.reconcile(Instant::now()).is_empty());
        assert_eq!(mgr.link_count(), 0);
    }

    #[test]
    fn inbound_cancels_pending_dial() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(AlwaysProven));
        let now = Instant::now();
        mgr.reconcile(now);
        assert!(mgr.has_link("b:1"));
        let cancelled = mgr.register_inbound("b:1", now);
        assert!(cancelled, "inbound for a peer we were dialing cancels the dial");
    }

    #[test]
    fn forward_local_enqueues_one_copy_per_peer() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1), ep("c", 1)]);
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(AlwaysProven));
        // create acceptor links for both peers (simulating established conns)
        let now = Instant::now();
        mgr.register_inbound("b:1", now);
        mgr.register_inbound("c:1", now);
        let local = RelayedPacket::local(audio_packet());
        let sent = mgr.forward_local(&local, "W");
        assert_eq!(sent, 2);
    }

    // Acceptor-side bidirectional writer seam: for an acceptor link
    // created by `register_inbound`, the receiver `take_outbound_receiver` yields
    // is the SAME queue `forward_local` enqueues onto. The accept path spawns a
    // write pump over this receiver, so a local-origin packet forwarded to the peer
    // is delivered back over the accepted connection (the live datagram send is the
    // documented socket boundary). Without this, accepted peers got no audio
    // (half-duplex).
    #[tokio::test]
    async fn acceptor_outbound_receiver_receives_forwarded_packets() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(AlwaysProven));
        // Acceptor link (as register_inbound creates on the accept path).
        mgr.register_inbound("b:1", Instant::now());

        // The write pump takes the receiver for this link.
        let mut rx = mgr
            .take_outbound_receiver("b:1")
            .expect("acceptor link must yield its outbound receiver");

        // forward_local enqueues onto the link's outbound queue.
        let local = RelayedPacket::local(audio_packet());
        assert_eq!(mgr.forward_local(&local, "W"), 1);

        // The taken receiver observes exactly that enqueue.
        let got = rx.recv().await.expect("forwarded packet must arrive on the taken receiver");
        assert_eq!(got.packet.packet_type, PacketType::AudioFrame);

        // Receiver is takeable once: a second take yields None.
        assert!(mgr.take_outbound_receiver("b:1").is_none());
    }

    #[test]
    fn forward_skips_local_only_world() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(AlwaysProven));
        // no peers registered for "W" -> two local clients sharing it relay nothing
        let local = RelayedPacket::local(audio_packet());
        assert_eq!(mgr.forward_local(&local, "W"), 0);
    }

    #[test]
    fn forward_never_relays_relayed_origin() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(AlwaysProven));
        mgr.register_inbound("b:1", Instant::now());
        let relayed = RelayedPacket::from_peer(audio_packet());
        assert_eq!(mgr.forward_local(&relayed, "W"), 0);
    }

    #[test]
    fn no_challenges_without_a_prover_installed() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(AlwaysProven));
        assert!(mgr.challenges_to_send(Instant::now()).is_empty());
    }

    #[test]
    fn challenges_emitted_for_worlds_with_unproven_peers() {
        use crate::services::relay::presence::PresenceProver;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let prover = PresenceProver::new_shared();
        // gate must be the same prover so is_proven and orchestration agree
        let mgr = PeerManager::new(ep("a", 1), table, sink, prover.clone());
        mgr.set_prover(prover);
        let challenges = mgr.challenges_to_send(Instant::now());
        assert_eq!(challenges.len(), 1);
        assert_eq!(challenges[0].0, "W");
        assert_eq!(challenges[0].1.token.len(), 32);
    }

    #[test]
    fn no_challenge_once_peer_proven() {
        use crate::services::relay::presence::PresenceProver;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let prover = PresenceProver::new_shared();
        let mgr = PeerManager::new(ep("a", 1), table, sink, prover.clone());
        mgr.set_prover(prover);
        let now = Instant::now();
        let challenges = mgr.challenges_to_send(now);
        let token = challenges[0].1.token.clone();
        // peer echoes our token over the link -> proven -> no further challenge
        mgr.route_observed_from_peer("b:1", &token, now);
        assert!(mgr.challenges_to_send(now).is_empty());
    }

    #[test]
    fn observed_from_peer_marks_gate_proven() {
        use crate::services::relay::presence::PresenceProver;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let prover = PresenceProver::new_shared();
        let mgr = PeerManager::new(ep("a", 1), table, sink, prover.clone());
        mgr.set_prover(prover.clone());
        let now = Instant::now();
        let token = mgr.challenges_to_send(now)[0].1.token.clone();
        // gate is MUTUAL: a peer echoing our token alone does not satisfy it.
        assert!(!mgr.presence.is_proven(&ep("b", 1), "W"));
        mgr.route_observed_from_peer("b:1", &token, now);
        // peer proved us (single direction), but the mutual gate is still closed
        // until we echo a token they injected.
        assert!(prover.peer_proved_us(&ep("b", 1), "W"));
        assert!(!mgr.presence.is_proven(&ep("b", 1), "W"));
        prover.record_echoed_to_peer("b:1", "W");
        assert!(mgr.presence.is_proven(&ep("b", 1), "W"));
    }

    #[test]
    fn local_observed_tokens_drain_for_echo() {
        use crate::services::relay::presence::PresenceProver;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let prover = PresenceProver::new_shared();
        let mgr = PeerManager::new(ep("a", 1), PeerTable::new_shared(), sink, prover.clone());
        mgr.set_prover(prover);
        let now = Instant::now();
        // the token must be a known (expected) challenge to be echoed
        mgr.expect_observed("peer-tok", "W", now);
        mgr.on_local_client_observed("peer-tok", now);
        assert_eq!(
            mgr.tokens_to_echo_to_peer(),
            vec![("peer-tok".to_string(), "W".to_string())]
        );
        assert!(mgr.tokens_to_echo_to_peer().is_empty());
    }

    // At the manager seam: an unknown observed token is dropped.
    #[test]
    fn local_observed_unknown_token_is_dropped() {
        use crate::services::relay::presence::PresenceProver;
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let prover = PresenceProver::new_shared();
        let mgr = PeerManager::new(ep("a", 1), PeerTable::new_shared(), sink, prover.clone());
        mgr.set_prover(prover);
        mgr.on_local_client_observed("garbage", Instant::now());
        assert!(mgr.tokens_to_echo_to_peer().is_empty());
    }

    #[test]
    fn sweep_closes_idle_links_for_bilateral_teardown() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let table = PeerTable::new_shared();
        let mgr = PeerManager::new(ep("a", 1), table, sink, Arc::new(AlwaysProven));
        let t0 = Instant::now();
        mgr.register_inbound("b:1", t0);
        // not yet idle
        assert!(mgr.sweep_idle(t0 + std::time::Duration::from_secs(299)).is_empty());
        let closed = mgr.sweep_idle(t0 + std::time::Duration::from_secs(301));
        assert_eq!(closed, vec!["b:1".to_string()]);
        // link removed after close
        assert!(!mgr.has_link("b:1"));
    }
}
