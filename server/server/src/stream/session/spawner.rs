use super::SessionLink;
use crate::stream::quic::connection_registry::{ConnectionRegistry, RoutedPacket};
use crate::stream::quic::stream_manager::{InputStream, OutputStream};
use crate::stream::quic::{CacheManager, PacketIdentityStamp, WebhookReceiver};
use common::s2n_quic::Connection;
use common::structs::packet::{
    PacketType, PlayerDataPacket, PlayerPositionPacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::traits::StreamTrait;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

/// Runs one voice session, whatever carried it.
///
/// Every routing, identity and cache decision a session makes lives here and nowhere else.
/// A transport's job ends at handing over a `SessionLink` and the identity its handshake
/// proved; from that point QUIC and WebSocket sessions are the same code, which is what
/// keeps the two from drifting apart packet type by packet type.
pub(crate) struct SessionSpawner {
    connection_registry: Arc<ConnectionRegistry>,
    cache_manager: CacheManager,
    broadcast_range: f32,
    deafen_distance: f32,
    webhook_receiver: WebhookReceiver,
}

impl SessionSpawner {
    // Bounded per session. A consumer that cannot keep up drops packets rather than
    // growing without limit, which is the trade the audio path is built around.
    const ROUTED_PACKET_CAPACITY: usize = 500;

    pub(crate) fn new(
        connection_registry: Arc<ConnectionRegistry>,
        cache_manager: CacheManager,
        broadcast_range: f32,
        deafen_distance: f32,
        webhook_receiver: WebhookReceiver,
    ) -> Self {
        Self {
            connection_registry,
            cache_manager,
            broadcast_range,
            deafen_distance,
            webhook_receiver,
        }
    }

    pub(crate) fn new_shared(
        connection_registry: Arc<ConnectionRegistry>,
        cache_manager: CacheManager,
        broadcast_range: f32,
        deafen_distance: f32,
        webhook_receiver: WebhookReceiver,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            connection_registry,
            cache_manager,
            broadcast_range,
            deafen_distance,
            webhook_receiver,
        ))
    }

    /// Serves one session until either direction ends.
    ///
    /// `peer_connection` is present only on an inbound peer link, which is server-to-server
    /// QUIC by nature. A player session passes `None` and the relay branch is unreachable
    /// for it.
    pub(crate) async fn run(
        &self,
        link: SessionLink,
        device: u64,
        player_identity: Option<String>,
        peer_endpoint: Option<String>,
        peer_connection: Option<Arc<Connection>>,
    ) {
        let label = player_identity
            .clone()
            .or_else(|| peer_endpoint.clone())
            .unwrap_or_else(|| format!("device {device}"));

        let (packet_tx, packet_rx) = mpsc::channel::<RoutedPacket>(Self::ROUTED_PACKET_CAPACITY);

        let mut input_stream = InputStream::new(Some(link.clone()), None);
        if let Some(identity) = &player_identity {
            input_stream.set_identity(identity.clone(), device);
        }
        let mut output_stream = OutputStream::new(Some(link));
        output_stream.set_packet_receiver(packet_rx);

        // Registers this session under its authenticated identity. Both the identity and
        // the device id are known before the first packet, so this runs at handshake
        // rather than being triggered by one.
        let register_connection = {
            let player_id_lock = output_stream.player_id.clone();
            let registry = self.connection_registry.clone();
            let tx = packet_tx.clone();
            move |identity: String| {
                if player_id_lock.set(identity.clone()).is_err() {
                    tracing::warn!("Player ID already set for connection");
                }
                registry.register(device, identity, tx.clone());
            }
        };

        input_stream.set_disconnect_callback(self.disconnect_callback(device));
        input_stream.set_webhook_receiver(self.webhook_receiver.clone());

        let (input_shutdown_tx, input_shutdown_rx) = oneshot::channel();
        let (output_shutdown_tx, output_shutdown_rx) = oneshot::channel();

        let input = self.run_input(
            input_stream,
            input_shutdown_rx,
            Box::new(register_connection),
            device,
            peer_connection,
            player_identity,
            peer_endpoint,
        );
        let input_task = tokio::spawn(input);

        let output_task = tokio::spawn(async move {
            if let Err(e) = Self::run_output(output_stream, output_shutdown_rx).await {
                tracing::error!("Output stream error: {}", e);
            }
        });

        tokio::select! {
            _ = input_task => { let _ = output_shutdown_tx.send(()); },
            _ = output_task => { let _ = input_shutdown_tx.send(()); }
        }

        tracing::info!("Session {} closed", label);
    }

    /// Unregisters the session and clears every cache keyed on its identity, then announces
    /// the channel departures that cleanup produced.
    fn disconnect_callback(&self, device: u64) -> Box<dyn Fn(String) + Send + Sync> {
        let cache_manager = self.cache_manager.clone();
        let webhook_receiver = self.webhook_receiver.clone();
        let registry = self.connection_registry.clone();

        Box::new(move |player_id: String| {
            let cache_manager = cache_manager.clone();
            let webhook_receiver = webhook_receiver.clone();
            let registry = registry.clone();

            tokio::spawn(async move {
                tracing::info!("Player {} (device: {}) disconnected", player_id, device);

                registry.unregister(device);

                match cache_manager.remove_player(&player_id).await {
                    Ok(removed_channels) => {
                        for channel_id in removed_channels {
                            let leave_packet = QuicNetworkPacket {
                                sender: Some(common::structs::packet::PacketSender::new(
                                    player_id.clone(),
                                    device,
                                )),
                                packet_type: PacketType::ChannelEvent,
                                data: QuicNetworkPacketData::ChannelEvent(
                                    common::structs::packet::ChannelEventPacket::new(
                                        common::structs::channel::ChannelEvents::Leave,
                                        player_id.clone(),
                                        channel_id.clone(),
                                    ),
                                ),
                                // Not a server fan-out, so this envelope carries no sequence.
                                ..Default::default()
                            };

                            if let Err(e) = webhook_receiver.send_packet(leave_packet).await {
                                tracing::error!(
                                    "Failed to broadcast channel leave event for player {} channel {}: {}",
                                    player_id,
                                    channel_id,
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "Broadcast channel leave event: player {} left channel {}",
                                    player_id,
                                    channel_id
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to remove player {}: {}", player_id, e);
                    }
                }
            });
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn run_input(
        &self,
        mut input_stream: InputStream,
        mut shutdown_rx: oneshot::Receiver<()>,
        register_connection: Box<dyn Fn(String) + Send + Sync>,
        device: u64,
        peer_connection: Option<Arc<Connection>>,
        player_identity: Option<String>,
        peer_endpoint: Option<String>,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        let connection_registry = self.connection_registry.clone();
        let cache_manager = self.cache_manager.clone();
        let broadcast_range = self.broadcast_range;
        let deafen_distance = self.deafen_distance;

        async move {
            let (packet_tx, mut packet_rx) = mpsc::unbounded_channel();
            input_stream.set_producer(packet_tx);

            let stream_task = tokio::spawn(async move { input_stream.start().await });

            let player_cache = cache_manager.players().inner_arc();
            // Identity is settled by the handshake before this loop starts. A player's
            // every inbound packet is stamped with its authenticated name; a peer server
            // feeds the relay ingest and is never stamped, because relayed packets carry
            // their original sender's identity single-hop.

            // Both keys the registry needs — the identity and the device id — come from
            // the transport itself, so nothing is waiting on the wire to reveal them.
            if let Some(identity) = &player_identity {
                register_connection(identity.clone());
                tracing::info!("Registered authenticated player identity: {identity}");
            }

            // An inbound peer link registers with the relay up front rather than waiting
            // for a first packet to reveal who it is, then drains its outbound queue back
            // onto this same connection so `forward_local`'s per-peer enqueues reach
            // acceptor-accepted peers. Mirrors the dialer's write pump.
            if let Some(endpoint) = &peer_endpoint {
                match connection_registry.peer_manager() {
                    Some(pm) => {
                        pm.register_inbound(endpoint, std::time::Instant::now());

                        // Taken only once a connection exists to drain it onto. Taking the
                        // receiver and then finding nothing to write with would consume the
                        // peer's only outbound queue and drop it.
                        if let Some(write_conn) = peer_connection.clone()
                            && let Some(mut outbound_rx) = pm.take_outbound_receiver(endpoint)
                        {
                            tokio::spawn(async move {
                                while let Some(relayed) = outbound_rx.recv().await {
                                    if let Ok(bytes) = relayed.packet.to_datagram() {
                                        let _ = write_conn.datagram_mut(
                                            |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                                                dg.send_datagram(bytes.into())
                                            },
                                        );
                                    }
                                }
                            });
                        }

                        tracing::info!(
                            "Accepted inbound peer connection: {} (relay ingest path)",
                            endpoint
                        );
                    }
                    None => {
                        tracing::warn!(
                            "Inbound peer-identity connection {} but no relay manager is wired; dropping",
                            endpoint
                        );
                    }
                }
            }

            loop {
                tokio::select! {
                    Some(server_packet) = packet_rx.recv() => {
                        let mut packet = server_packet.data;

                        // Stamp the certificate identity and this session's device id before
                        // anything downstream reads either.
                        if let Some(identity) = &player_identity {
                            PacketIdentityStamp::apply(&mut packet, identity, device);
                        }

                        // Inbound peer link: route every packet straight into the
                        // relay ingest (FromPeer) — single-hop, registration
                        // bypassed. Never touches the local client/broadcast path.
                        if let Some(endpoint) = &peer_endpoint {
                            if let Some(pm) = connection_registry.peer_manager() {
                                pm.ingest(endpoint, packet).await;
                            }
                            continue;
                        }

                        // process_packet has no AudioFrame arm; skipping it avoids a
                        // full packet clone (audio payload included) per frame.
                        if packet.packet_type != PacketType::AudioFrame {
                            if let Err(e) = cache_manager
                                .process_packet(packet.clone())
                                .await
                            {
                                tracing::error!("Failed to process packet in cache manager: {}", e);
                            }
                        }

                        let updated_packet = if packet.packet_type == PacketType::AudioFrame {
                            match cache_manager.update_coordinates(packet).await {
                                Ok(updated) => updated,
                                Err(e) => {
                                    tracing::error!("Failed to update coordinates: {}", e);
                                    continue;
                                }
                            }
                        } else {
                            packet
                        };

                        match updated_packet.packet_type {
                            PacketType::AudioFrame => {
                                // Local-origin audio: forward to peer servers
                                // sharing the sender's relay world (single-hop;
                                // relayed-origin packets never reach this path).
                                connection_registry.forward_local_to_peers(&updated_packet);
                                connection_registry
                                    .route_audio_frame(&updated_packet, &player_cache, broadcast_range, deafen_distance)
                                    .await;
                            }
                            PacketType::PlayerPosition => {
                                if let QuicNetworkPacketData::PlayerPosition(PlayerPositionPacket {
                                    player,
                                }) = updated_packet.data.clone()
                                {
                                    // A proxy client self-reports its position and
                                    // relies on this echo to anchor its own
                                    // listener, so the packet still goes out -- but
                                    // only to the player it describes.
                                    let echo = QuicNetworkPacket {
                                        packet_type: PacketType::PlayerData,
                                        sender: updated_packet.sender.clone(),
                                        data: QuicNetworkPacketData::PlayerData(
                                            PlayerDataPacket::new(vec![player]),
                                        ),
                                        // Not a server fan-out, so this envelope carries no sequence.
                                        ..Default::default()
                                    };
                                    connection_registry.send_positions_to_owners(&echo);
                                }
                            }
                            PacketType::PeerPresenceObserved => {
                                // A local client reported a `!bvcp` code observed in the
                                // realm. Route it to the asker-side observe handler
                                // (Flow 1) to redeem against the offering minter and open
                                // the peer link. Never broadcast onward.
                                if let QuicNetworkPacketData::PeerPresenceObserved(observed) =
                                    updated_packet.data
                                {
                                    connection_registry.on_peer_presence_observed(observed.token);
                                }
                            }
                            PacketType::PeerAnnounceObserved => {
                                // A local client reported a peer `!bvca` announce observed
                                // in the realm. Record the peer endpoint for the observer's
                                // world so the offer/forward paths can reach it. Never
                                // broadcast onward.
                                if let QuicNetworkPacketData::PeerAnnounceObserved(announce) =
                                    updated_packet.data
                                {
                                    connection_registry
                                        .on_peer_announce_observed(announce.hashed_world, announce.endpoint);
                                }
                            }
                            PacketType::PlayerData => {
                                // Clients report position as PlayerPosition, so a
                                // clientbound-shaped PlayerData arriving here is not
                                // something the client should be sending. Address it
                                // per-player like every other position packet rather
                                // than letting one connection push coordinates to
                                // everyone.
                                connection_registry.send_positions_to_owners(&updated_packet);
                            }
                            PacketType::QueryState
                            | PacketType::PlayerPreference
                            | PacketType::ClientAction => {
                                // All three are consumed by cache_manager's
                                // process_packet (QueryState/PlayerPreference are
                                // cached; serverbound group ClientActions route
                                // through ClientActionService). None are broadcast.
                            }
                            _ => {
                                connection_registry.broadcast_to_all(updated_packet);
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        tracing::info!("Input stream received shutdown signal");
                        break;
                    }
                }
            }

            let _ = stream_task.await;
        }
    }

    async fn run_output(
        mut output_stream: OutputStream,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<(), anyhow::Error> {
        tokio::select! {
            result = output_stream.start() => {
                if let Err(e) = result {
                    tracing::error!("Output stream error: {}", e);
                }
            }
            _ = &mut shutdown_rx => {
                tracing::info!("Output stream received shutdown signal");
                if let Err(e) = output_stream.stop().await {
                    tracing::error!("Error stopping output stream: {}", e);
                }
            }
        }

        Ok(())
    }
}
