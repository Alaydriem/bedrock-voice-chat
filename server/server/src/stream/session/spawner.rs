use common::curia;
use super::SessionLink;
use crate::stream::quic::connection::{AtCapacity, ConnectionRegistry, RoutedPacket};
use crate::stream::quic::stream_manager::{InputStream, OutputStream};
use crate::stream::quic::{CacheManager, PacketIdentityStamp, WebhookReceiver};
use common::structs::packet::{
    PacketType, PlayerDataPacket, PlayerPositionPacket, QuicNetworkPacket, QuicNetworkPacketData,
    ServerErrorPacket, ServerErrorType,
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
    // Microseconds each session's output loop waits to batch outbound datagrams.
    send_batch_wait_micros: u64,
}

impl SessionSpawner {
    // Bounded per session. A consumer that cannot keep up drops packets rather than
    // growing without limit, which is the trade the audio path is built around.
    const ROUTED_PACKET_CAPACITY: usize = 500;

    // How long a refused session stays open so its refusal datagram can reach the wire.
    // Paid only by a connection that is being turned away, and generous against the
    // sub-millisecond flush a local transport performs.
    const REFUSAL_FLUSH_GRACE: std::time::Duration = std::time::Duration::from_millis(250);

    pub(crate) fn new(
        connection_registry: Arc<ConnectionRegistry>,
        cache_manager: CacheManager,
        broadcast_range: f32,
        deafen_distance: f32,
        webhook_receiver: WebhookReceiver,
        send_batch_wait_micros: u64,
    ) -> Self {
        Self {
            connection_registry,
            cache_manager,
            broadcast_range,
            deafen_distance,
            webhook_receiver,
            send_batch_wait_micros,
        }
    }

    pub(crate) fn new_shared(
        connection_registry: Arc<ConnectionRegistry>,
        cache_manager: CacheManager,
        broadcast_range: f32,
        deafen_distance: f32,
        webhook_receiver: WebhookReceiver,
        send_batch_wait_micros: u64,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            connection_registry,
            cache_manager,
            broadcast_range,
            deafen_distance,
            webhook_receiver,
            send_batch_wait_micros,
        ))
    }

    /// Serves one session until either direction ends.
    ///
    pub(crate) async fn run(
        &self,
        link: SessionLink,
        device: u64,
        player_identity: Option<String>,
        // The certificate this session's handshake proved. Carried so a revocation can
        // close the session it opened rather than every session the identity holds.
        fingerprint: String,
    ) {
        let label = player_identity
            .clone()
            .unwrap_or_else(|| format!("device {device}"));

        let (packet_tx, packet_rx) = mpsc::channel::<RoutedPacket>(Self::ROUTED_PACKET_CAPACITY);

        let refusal_link = link.clone();
        let mut input_stream = InputStream::new(Some(link.clone()), None);
        if let Some(identity) = &player_identity {
            input_stream.set_identity(identity.clone(), device);
        }
        let mut output_stream = OutputStream::new(Some(link), self.send_batch_wait_micros);
        output_stream.set_packet_receiver(packet_rx);

        // Registers this session under its authenticated identity. Both the identity and
        // the device id are known before the first packet, so this runs at handshake
        // rather than being triggered by one.
        let register_connection = {
            let player_id_lock = output_stream.player_id.clone();
            let registry = self.connection_registry.clone();
            let tx = packet_tx.clone();
            move |identity: String| -> Result<(), AtCapacity> {
                // Converted once here, at handshake, rather than per frame in the fan-out.
                let shared: std::sync::Arc<str> = std::sync::Arc::from(identity.as_str());
                registry.try_register(device, shared, fingerprint.clone(), tx.clone())?;
                if player_id_lock.set(identity).is_err() {
                    curia::warn!("Player ID already set for connection");
                }
                Ok(())
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
            player_identity,
            refusal_link,
        );
        let input_task = tokio::spawn(input);

        let output_task = tokio::spawn(async move {
            if let Err(e) = Self::run_output(output_stream, output_shutdown_rx).await {
                curia::error!("Output stream error: {}", e);
            }
        });

        tokio::select! {
            _ = input_task => { let _ = output_shutdown_tx.send(()); },
            _ = output_task => { let _ = input_shutdown_tx.send(()); }
        }

        curia::info!("Session {} closed", label);
    }

    /// Tells a refused client why, on the link its handshake established.
    ///
    /// A transport-level close carries no reason a client can read, so it cannot tell a full
    /// server from a revoked credential. This mirrors the version refusal, which answers on
    /// the datagram path for the same reason.
    async fn refuse_at_capacity(link: &SessionLink, limit: u32) {
        let packet = QuicNetworkPacket {
            packet_type: PacketType::ServerError,
            data: QuicNetworkPacketData::ServerError(ServerErrorPacket {
                error_type: ServerErrorType::AtCapacity { limit },
                message: format!("This server is full ({limit} connections). Retrying shortly."),
            }),
            // Not a server fan-out, so this envelope carries no sequence.
            ..Default::default()
        };

        if let Ok(bytes) = packet.to_datagram() {
            let _ = link.send(bytes::Bytes::from(bytes));
            // `send` only queues the datagram with the transport; the flush happens after it
            // returns. Every other refusal on this path breaks out of a live session loop, so
            // the connection outlives the queue by itself. This one is the whole session, and
            // returning drops the connection — which discarded the datagram and left the
            // client with an unexplained close.
            tokio::time::sleep(Self::REFUSAL_FLUSH_GRACE).await;
        }
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
                curia::info!("Player {} (device: {}) disconnected", player_id, device);

                registry.unregister(device);

                // The callback is handed the identity as text by the transport. A value that
                // does not parse belongs to no player, so there is nothing to evict.
                let Ok(identity) = player_id.parse::<common::PlayerIdentity>() else {
                    curia::error!(format!("Disconnect for a non-canonical identity: {player_id}"));
                    return;
                };

                match cache_manager.remove_player(&identity).await {
                    Ok(removed_channels) => {
                        for (channel_id, creator) in removed_channels {
                            let leave_packet = QuicNetworkPacket {
                                sender: Some(common::structs::packet::PacketSender::player(
                                    identity.clone(),
                                    device,
                                )),
                                packet_type: PacketType::ChannelEvent,
                                data: QuicNetworkPacketData::ChannelEvent(
                                    common::structs::packet::ChannelEventPacket::new(
                                        common::structs::channel::ChannelEvents::Leave,
                                        identity.clone(),
                                        channel_id.clone(),
                                        None,
                                        Some(creator),
                                    ),
                                ),
                                // Not a server fan-out, so this envelope carries no sequence.
                                ..Default::default()
                            };

                            if let Err(e) = webhook_receiver.send_packet(leave_packet).await {
                                curia::error!(
                                    "Failed to broadcast channel leave event for player {} channel {}: {}",
                                    identity,
                                    channel_id,
                                    e
                                );
                            } else {
                                curia::info!(
                                    "Broadcast channel leave event: player {} left channel {}",
                                    identity,
                                    channel_id
                                );
                            }
                        }
                    }
                    Err(e) => {
                        curia::error!("Failed to remove player {}: {}", identity, e);
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
        register_connection: Box<dyn Fn(String) -> Result<(), AtCapacity> + Send + Sync>,
        device: u64,
        player_identity: Option<String>,
        refusal_link: SessionLink,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        let connection_registry = self.connection_registry.clone();
        let cache_manager = self.cache_manager.clone();
        let broadcast_range = self.broadcast_range;
        let deafen_distance = self.deafen_distance;

        async move {
            let (packet_tx, mut packet_rx) = mpsc::unbounded_channel();
            input_stream.set_producer(packet_tx);

            // Both keys the registry needs — the identity and the device id — come from
            // the transport itself, so nothing is waiting on the wire to reveal them. A
            // refused session returns before the input stream starts, so it never reads a
            // packet it would have to route with no registration behind it.
            if let Some(identity) = &player_identity {
                match register_connection(identity.clone()) {
                    Ok(()) => {
                        curia::info!(format!("Registered authenticated player identity: {identity}"))
                    }
                    Err(refusal) => {
                        curia::info!(format!("Refused {identity}: {refusal}"));
                        Self::refuse_at_capacity(&refusal_link, refusal.limit).await;
                        return;
                    }
                }
            }

            let stream_task = tokio::spawn(async move { input_stream.start().await });

            let player_cache = cache_manager.players().inner_arc();
            // Identity is settled by the handshake before this loop starts. A player's
            // every inbound packet is stamped with its authenticated name; a peer server
            // feeds the relay ingest and is never stamped, because relayed packets carry
            // their original sender's identity single-hop.

            loop {
                tokio::select! {
                    Some(server_packet) = packet_rx.recv() => {
                        let mut packet = server_packet.data;

                        // Stamp the certificate identity and this session's device id before
                        // anything downstream reads either.
                        if let Some(identity) = &player_identity {
                            PacketIdentityStamp::apply(&mut packet, identity, device);
                        }

                        // process_packet has no AudioFrame arm; skipping it avoids a
                        // full packet clone (audio payload included) per frame.
                        if packet.packet_type != PacketType::AudioFrame {
                            if let Err(e) = cache_manager
                                .process_packet(packet.clone())
                                .await
                            {
                                curia::error!("Failed to process packet in cache manager: {}", e);
                            }
                        }

                        let mut updated_packet = packet;

                        match updated_packet.packet_type {
                            PacketType::AudioFrame => {
                                // Resolved once and handed to both, so the relay and the
                                // fan-out cannot disagree about where a speaker is.
                                let speaker =
                                    cache_manager.resolve_speaker(&updated_packet).await;
                                cache_manager
                                    .attach_speaker(&mut updated_packet, speaker.as_ref());

                                connection_registry
                                    .forward_local_to_peers(&updated_packet, speaker.as_ref());
                                connection_registry
                                    .route_audio_frame(
                                        &updated_packet,
                                        speaker.as_ref(),
                                        &player_cache,
                                        broadcast_range,
                                        deafen_distance,
                                    )
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
                        curia::info!("Input stream received shutdown signal");
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
                    curia::error!("Output stream error: {}", e);
                }
            }
            _ = &mut shutdown_rx => {
                curia::info!("Output stream received shutdown signal");
                if let Err(e) = output_stream.stop().await {
                    curia::error!("Error stopping output stream: {}", e);
                }
            }
        }

        Ok(())
    }
}
