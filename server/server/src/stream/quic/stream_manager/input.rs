use crate::stream::quic::{ServerInputPacket, WebhookReceiver};
use anyhow::Error;
use bytes::Bytes;
use common::s2n_quic::Connection;
use common::structs::packet::{
    ConnectionEventType, PacketSender, PacketType, PlayerPresenceEvent, QuicNetworkPacket,
    QuicNetworkPacketData, ServerErrorPacket, ServerErrorType,
};
use common::traits::StreamTrait;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use moka::sync::Cache;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;

// Minimal Future wrapper to await a single datagram without external crates
struct RecvDatagram<'c> {
    conn: &'c Connection,
}
impl<'c> RecvDatagram<'c> {
    fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }
}
impl<'c> Future for RecvDatagram<'c> {
    type Output = Result<Bytes, anyhow::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.conn.datagram_mut(
            |r: &mut common::s2n_quic::provider::datagram::default::Receiver| {
                r.poll_recv_datagram(cx)
            },
        ) {
            Ok(Poll::Ready(Ok(bytes))) => Poll::Ready(Ok(bytes)),
            Ok(Poll::Ready(Err(e))) => Poll::Ready(Err(anyhow::anyhow!(e))),
            Ok(Poll::Pending) => Poll::Pending,
            Err(e) => Poll::Ready(Err(anyhow::anyhow!(e))),
        }
    }
}

async fn recv_one_datagram(conn: &Connection) -> Result<Bytes, anyhow::Error> {
    RecvDatagram::new(conn).await
}

pub(crate) struct InputStream {
    connection: Option<Arc<Connection>>,
    // Producer to send received data to other components
    producer: Option<mpsc::UnboundedSender<ServerInputPacket>>,
    is_stopped: Arc<AtomicBool>,
    // This connection's authenticated identity and device id, both settled from the mTLS
    // certificate before the first datagram arrives. Absent on a peer link, which carries
    // other servers' speakers rather than having an identity of its own.
    identity: Option<String>,
    device: Option<u64>,
    // Per-speaker last seen audio timestamp cache (ms since epoch), keyed on canonical
    // identity. A peer link carries many speakers, so this cannot be per-connection.
    last_seen_ts: Cache<String, i64>,
    // Callback to notify when disconnect happens (for cache cleanup)
    disconnect_callback: Option<Box<dyn Fn(String) + Send + Sync>>,
    // Webhook receiver for sending presence events
    webhook_receiver: Option<WebhookReceiver>,
}

impl InputStream {
    const LARGE_JUMP_FORWARD_MS: i64 = 3_000;

    // Hard cap on the per-speaker last-seen timestamp cache. A peer link's speakers are
    // named by the peer, so without a capacity a misbehaving peer relaying unique identities
    // could grow this without bound. Benchmarked concurrency headroom is ~10K clients; 100K
    // bounds memory far above that.
    const LAST_SEEN_MAX_CAPACITY: u64 = 100_000;

    pub fn new(
        connection: Option<Arc<Connection>>,
        producer: Option<mpsc::UnboundedSender<ServerInputPacket>>,
    ) -> Self {
        // 15-minute idle eviction plus a hard capacity so an untrusted stream of
        // unique identities cannot exhaust memory.
        let last_seen_ts = Cache::builder()
            .time_to_idle(Duration::from_secs(15 * 60))
            .max_capacity(Self::LAST_SEEN_MAX_CAPACITY)
            .build();

        Self {
            connection,
            producer,
            is_stopped: Arc::new(AtomicBool::new(true)),
            identity: None,
            device: None,
            last_seen_ts,
            disconnect_callback: None,
            webhook_receiver: None,
        }
    }

    pub fn set_producer(&mut self, producer: mpsc::UnboundedSender<ServerInputPacket>) {
        self.producer = Some(producer);
    }

    pub fn set_disconnect_callback(&mut self, callback: Box<dyn Fn(String) + Send + Sync>) {
        self.disconnect_callback = Some(callback);
    }

    /// Declares whose connection this is, from the mTLS certificate.
    ///
    /// Called at accept, before the stream starts, so nothing here has to infer an identity
    /// from a datagram. A peer link is left unset: it carries other servers' speakers.
    pub fn set_identity(&mut self, identity: String, device: u64) {
        self.identity = Some(identity);
        self.device = Some(device);
    }

    pub fn set_webhook_receiver(&mut self, webhook_receiver: WebhookReceiver) {
        self.webhook_receiver = Some(webhook_receiver);
    }

    pub async fn send_event(&self, packet: QuicNetworkPacket) {
        if let Some(webhook_receiver) = &self.webhook_receiver {
            let webhook_receiver_clone = webhook_receiver.clone();
            tokio::spawn(async move {
                if let Err(e) = webhook_receiver_clone.send_packet(packet).await {
                    tracing::error!("Failed to send player connected event: {}", e);
                }
            });
        }
    }

    // Who this frame's ordering is tracked against. A relayed frame already carries the
    // originating server's stamp, which names the real speaker; a local player's does not
    // yet, and is this connection's own identity. Neither case reads a client's claim.
    fn speaker_key(&self, packet: &QuicNetworkPacket) -> Option<String> {
        packet
            .sender_identity()
            .map(str::to_string)
            .or_else(|| self.identity.clone())
    }

    fn decide_accept(last_seen: Option<i64>, ts: i64, jump_threshold_ms: i64) -> (bool, bool) {
        match last_seen {
            None => (true, false),
            Some(prev) => {
                if ts <= prev {
                    return (false, false);
                }
                let delta = ts - prev;
                (true, delta > jump_threshold_ms)
            }
        }
    }
}

impl StreamTrait for InputStream {
    fn is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }

    async fn stop(&mut self) -> Result<(), Error> {
        tracing::info!("Stopping QUIC input stream");
        self.is_stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn start(&mut self) -> Result<(), Error> {
        tracing::info!("Starting QUIC input stream");
        self.is_stopped.store(false, Ordering::Relaxed);

        if let (Some(connection), Some(producer)) = (self.connection.clone(), self.producer.clone())
        {
            let mut announced_presence = false;
            // Handle incoming datagrams from this connection
            loop {
                if self.is_stopped() {
                    break;
                }
                // Custom future to await a single datagram without futures crate
                let datagram = recv_one_datagram(&connection).await;
                match datagram {
                    Ok(bytes) => {
                        match QuicNetworkPacket::from_datagram(&bytes) {
                            Ok(packet) => {
                                match packet.packet_type {
                                    PacketType::AudioFrame => {
                                        // Use reference to avoid cloning data unnecessarily
                                        let ts_opt = match &packet.data {
                                            QuicNetworkPacketData::AudioFrame(af) => {
                                                Some(af.timestamp())
                                            }
                                            _ => None,
                                        };

                                        if let (Some(ts), Some(key)) =
                                            (ts_opt, self.speaker_key(&packet))
                                        {
                                            let last_seen = self.last_seen_ts.get(&key);
                                            let (accept, large_jump) = Self::decide_accept(
                                                last_seen,
                                                ts,
                                                Self::LARGE_JUMP_FORWARD_MS,
                                            );
                                            if !accept {
                                                if let Some(prev) = last_seen {
                                                    tracing::trace!(
                                                        "Dropping out-of-order AudioFrame: ts={} <= last_seen={}",
                                                        ts,
                                                        prev
                                                    );
                                                }
                                                continue; // Drop older/same-timestamp frame
                                            }

                                            // Update last seen timestamp for this speaker
                                            self.last_seen_ts.insert(key.clone(), ts);
                                            if large_jump {
                                                let prev = last_seen.unwrap_or(0);
                                                let delta = ts - prev;
                                                // Use a dedicated tracing target so this can be scraped/tapped later
                                                tracing::debug!(target: "ofo", "large_jump_forward speaker={} ts={} last_seen={} delta_ms={}", key, ts, prev, delta);
                                            }
                                        }
                                    }
                                    PacketType::Debug => match &packet.data {
                                        QuicNetworkPacketData::Debug(d) => {
                                            if let (Ok(client_version), Ok(server_version)) = (
                                                semver::Version::parse(&d.version),
                                                semver::Version::parse(
                                                    common::consts::version::PROTOCOL_VERSION,
                                                ),
                                            ) {
                                                // Reject if client major.minor is older than server
                                                if client_version.major < server_version.major
                                                    || (client_version.major
                                                        == server_version.major
                                                        && client_version.minor
                                                            < server_version.minor)
                                                {
                                                    let error_packet = ServerErrorPacket {
                                                        error_type: ServerErrorType::VersionIncompatible {
                                                            client_version: d.version.clone(),
                                                            server_version: common::consts::version::PROTOCOL_VERSION.to_string()
                                                        },
                                                        message: format!(
                                                            "Client version {} is too old. Server requires {}+. Please update your client.",
                                                            &d.version, common::consts::version::PROTOCOL_VERSION
                                                        )
                                                    };

                                                    let error_net = QuicNetworkPacket {
                                                        packet_type: PacketType::ServerError,
                                                        data: QuicNetworkPacketData::ServerError(
                                                            error_packet,
                                                        ),
                                                                                                            // Not a server fan-out, so this envelope carries no sequence.
                                                        ..Default::default()
                                                    };
                                                    if let Ok(bytes) = error_net.to_datagram() {
                                                        let _ = connection.datagram_mut(
                                                            |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                                                                dg.send_datagram(Bytes::from(bytes))
                                                            },
                                                        );
                                                    }

                                                    break;
                                                }
                                            }
                                        }
                                        _ => {}
                                    },
                                    PacketType::HealthCheck => {
                                        if let Ok(bytes) = packet.to_datagram() {
                                            let _ = connection.datagram_mut(|dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                                                dg.send_datagram(Bytes::from(bytes))
                                            });
                                        }
                                        continue;
                                    }
                                    _ => {}
                                };

                                // Announced on the first datagram rather than at accept, because
                                // this is the point the connection is proven to carry traffic — a
                                // handshake that never speaks is not a player who joined.
                                if !announced_presence {
                                    if let Some(identity) = &self.identity {
                                        announced_presence = true;
                                        tracing::info!("Player identity active: {identity}");

                                        self.send_event(QuicNetworkPacket {
                                            sender: Some(PacketSender::synthetic(
                                                PacketSender::SERVER_API,
                                            )),
                                            packet_type: PacketType::PlayerPresence,
                                            data: QuicNetworkPacketData::PlayerPresence(
                                                PlayerPresenceEvent {
                                                    player_name: identity.clone(),
                                                    timestamp: std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_millis()
                                                        as i64,
                                                    event_type: ConnectionEventType::Connected,
                                                },
                                            ),
                                            // Not a server fan-out, so this envelope carries no sequence.
                                            ..Default::default()
                                        })
                                        .await;
                                    }
                                }

                                let server_packet = ServerInputPacket { data: packet };
                                if let Err(e) = producer.send(server_packet) {
                                    tracing::error!("Failed to send packet to producer: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse QUIC datagram packet: {}", e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        let emsg = e.to_string();
                        let player = self.identity.clone().unwrap_or_else(|| "unknown".into());
                        let device = self.device.unwrap_or_else(|| connection.id());

                        // Treat connection-closed-like errors as fatal and close
                        let lower = emsg.to_ascii_lowercase();
                        let is_closed = (lower.contains("connection") && lower.contains("clos"))
                            || lower.contains("closed")
                            || lower.contains("reset");
                        if is_closed {
                            tracing::error!(
                                "datagram_recv_closed player={} device={} err={}",
                                player,
                                device,
                                emsg
                            );
                        } else {
                            tracing::error!(
                                "datagram_recv_error player={} device={} err={}",
                                player,
                                device,
                                emsg
                            );
                        }
                        break;
                    }
                }
            }

            // Handle disconnect / cleanup once loop exits
            if let (Some(callback), Some(identity), Some(device)) =
                (&self.disconnect_callback, &self.identity, self.device)
            {
                callback(identity.clone());
                if let Some(webhook_receiver) = &self.webhook_receiver {
                    let webhook_receiver_clone = webhook_receiver.clone();
                    let player_name = identity.clone();
                    tokio::spawn(async move {
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as i64;
                        let presence_packet = QuicNetworkPacket {
                            sender: Some(PacketSender::new(player_name.clone(), device)),
                            packet_type: PacketType::PlayerPresence,
                            data: QuicNetworkPacketData::PlayerPresence(PlayerPresenceEvent {
                                player_name: player_name.clone(),
                                timestamp,
                                event_type: ConnectionEventType::Disconnected,
                            }),
                                                    // Not a server fan-out, so this envelope carries no sequence.
                            ..Default::default()
                        };
                        if let Err(e) = webhook_receiver_clone.send_packet(presence_packet).await {
                            tracing::error!("Failed to send player disconnected event: {}", e);
                        }
                        tracing::debug!("Broadcast player disconnected event {}", player_name);
                    });
                }
            }
        }

        self.is_stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn metadata(&mut self, key: String, value: String) -> Result<(), Error> {
        tracing::info!(
            "Setting metadata for QUIC input stream: {} = {}",
            key,
            value
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::InputStream;

    #[test]
    fn test_decide_accept_none_prev() {
        let (accept, large) =
            InputStream::decide_accept(None, 100, InputStream::LARGE_JUMP_FORWARD_MS);
        assert!(accept);
        assert!(!large);
    }

    #[test]
    fn test_decide_accept_older_or_equal() {
        let (a1, l1) =
            InputStream::decide_accept(Some(100), 99, InputStream::LARGE_JUMP_FORWARD_MS);
        assert!(!a1);
        assert!(!l1);
        let (a2, l2) =
            InputStream::decide_accept(Some(100), 100, InputStream::LARGE_JUMP_FORWARD_MS);
        assert!(!a2);
        assert!(!l2);
    }

    #[test]
    fn test_decide_accept_newer_small_delta() {
        let (accept, large) =
            InputStream::decide_accept(Some(100), 150, InputStream::LARGE_JUMP_FORWARD_MS);
        assert!(accept);
        assert!(!large);
    }

    #[test]
    fn test_decide_accept_large_jump() {
        let (accept, large) = InputStream::decide_accept(
            Some(1000),
            1000 + InputStream::LARGE_JUMP_FORWARD_MS + 1,
            InputStream::LARGE_JUMP_FORWARD_MS,
        );
        assert!(accept);
        assert!(large);
    }
}
