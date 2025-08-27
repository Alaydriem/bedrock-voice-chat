use crate::stream::quic::{ServerInputPacket, WebhookReceiver};
use anyhow::Error;
use bytes::Bytes;
use common::structs::packet::{
    ConnectionEventType, PacketOwner, PacketType, PlayerPresenceEvent, QuicNetworkPacket,
    QuicNetworkPacketData, ServerErrorPacket, ServerErrorType,
};
use common::traits::StreamTrait;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use moka::sync::Cache;
use s2n_quic::Connection;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
        match self
            .conn
            .datagram_mut(|r: &mut s2n_quic::provider::datagram::default::Receiver| {
                r.poll_recv_datagram(cx)
            }) {
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

/// Helper function to create a short hash representation of client_id
fn client_id_hash(client_id: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    client_id.hash(&mut hasher);
    format!("{:x}", hasher.finish() & 0xFFFF) // Take only last 4 hex digits for readability
}

pub(crate) struct InputStream {
    connection: Option<Arc<Connection>>,
    // Producer to send received data to other components
    producer: Option<mpsc::UnboundedSender<ServerInputPacket>>,
    is_stopped: Arc<AtomicBool>,
    // Player identity from first packet with owner
    player_id: Option<String>,
    client_id: Option<Vec<u8>>,
    // Per-sender last seen audio timestamp cache (ms since epoch)
    last_seen_ts: Cache<Vec<u8>, i64>,
    // Callback to notify when disconnect happens (for cache cleanup)
    // Parameters: (player_name, client_id)
    disconnect_callback: Option<Box<dyn Fn(String, Vec<u8>) + Send + Sync>>,
    // Webhook receiver for sending presence events
    webhook_receiver: Option<WebhookReceiver>,
}

impl InputStream {
    const LARGE_JUMP_FORWARD_MS: i64 = 3_000;

    pub fn new(
        connection: Option<Arc<Connection>>,
        producer: Option<mpsc::UnboundedSender<ServerInputPacket>>,
    ) -> Self {
        // 15-minute time-to-idle per earlier plan
        let last_seen_ts = Cache::builder()
            .time_to_idle(Duration::from_secs(15 * 60))
            .build();

        Self {
            connection,
            producer,
            is_stopped: Arc::new(AtomicBool::new(true)),
            player_id: None,
            client_id: None,
            last_seen_ts,
            disconnect_callback: None,
            webhook_receiver: None,
        }
    }

    pub fn set_producer(&mut self, producer: mpsc::UnboundedSender<ServerInputPacket>) {
        self.producer = Some(producer);
    }

    pub fn set_disconnect_callback(
        &mut self,
        callback: Box<dyn Fn(String, Vec<u8>) + Send + Sync>,
    ) {
        self.disconnect_callback = Some(callback);
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

    fn sender_key_for_packet(&self, packet: &QuicNetworkPacket, conn: &Connection) -> Vec<u8> {
        if let Some(owner) = &packet.owner {
            if !owner.client_id.is_empty() {
                return owner.client_id.clone();
            }
        }

        if let Some(cid) = &self.client_id {
            if !cid.is_empty() {
                return cid.clone();
            }
        }

        let dbg_id = format!("{:?}", conn.id());
        let mut hasher = DefaultHasher::new();
        dbg_id.hash(&mut hasher);
        let h = hasher.finish();
        h.to_be_bytes().to_vec()
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

                                        if let Some(ts) = ts_opt {
                                            let key =
                                                self.sender_key_for_packet(&packet, &connection);
                                            let last_seen = self.last_seen_ts.get(&key);
                                            let (accept, large_jump) = Self::decide_accept(
                                                last_seen,
                                                ts,
                                                Self::LARGE_JUMP_FORWARD_MS,
                                            );
                                            if !accept {
                                                if let Some(prev) = last_seen {
                                                    tracing::trace!("Dropping out-of-order AudioFrame: ts={} <= last_seen={}", ts, prev);
                                                }
                                                continue; // Drop older/same-timestamp frame
                                            }

                                            // Update last seen timestamp for sender
                                            self.last_seen_ts.insert(key.clone(), ts);
                                            if large_jump {
                                                let client_hash = match &self.client_id {
                                                    Some(cid) => client_id_hash(cid),
                                                    None => {
                                                        // If we don't know the client yet, hash the derived key for some stability
                                                        client_id_hash(&key)
                                                    }
                                                };
                                                let prev = last_seen.unwrap_or(0);
                                                let delta = ts - prev;
                                                // Use a dedicated tracing target so this can be scraped/tapped later
                                                tracing::debug!(target: "ofo", "large_jump_forward client={} ts={} last_seen={} delta_ms={}", client_hash, ts, prev, delta);
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

                                                    self.send_event(QuicNetworkPacket {
                                                        owner: packet.owner.clone(),
                                                        packet_type: PacketType::ServerError,
                                                        data: QuicNetworkPacketData::ServerError(
                                                            error_packet,
                                                        ),
                                                    })
                                                    .await;

                                                    break;
                                                }
                                            }
                                        }
                                        _ => {}
                                    },
                                    _ => {}
                                };

                                if self.player_id.is_none() && packet.owner.is_some() {
                                    let owner = packet.owner.as_ref().unwrap();
                                    self.player_id = Some(owner.name.clone());
                                    self.client_id = Some(owner.client_id.clone());
                                    let client_hash = client_id_hash(&owner.client_id);
                                    tracing::info!(
                                        "Initialized player identity: {} (client: {})",
                                        owner.name,
                                        client_hash
                                    );

                                    self.send_event(QuicNetworkPacket {
                                        owner: Some(PacketOwner {
                                            name: String::from("api"),
                                            client_id: vec![],
                                        }),
                                        packet_type: PacketType::PlayerPresence,
                                        data: QuicNetworkPacketData::PlayerPresence(
                                            PlayerPresenceEvent {
                                                player_name: owner.name.clone(),
                                                timestamp: std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_millis()
                                                    as i64,
                                                event_type: ConnectionEventType::Connected,
                                            },
                                        ),
                                    })
                                    .await;
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
                        let player = self.player_id.clone().unwrap_or_else(|| "unknown".into());
                        let client_hash = self
                            .client_id
                            .as_ref()
                            .map(|cid| super::super::client_id_hash(cid))
                            .unwrap_or_else(|| {
                                // derive from connection id
                                let dbg_id = format!("{:?}", connection.id());
                                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                                use std::hash::{Hash, Hasher};
                                dbg_id.hash(&mut hasher);
                                format!("{:x}", hasher.finish() & 0xFFFF)
                            });

                        // Treat connection-closed-like errors as fatal and close
                        let lower = emsg.to_ascii_lowercase();
                        let is_closed = (lower.contains("connection") && lower.contains("clos"))
                            || lower.contains("closed")
                            || lower.contains("reset");
                        if is_closed {
                            tracing::error!(
                                "datagram_recv_closed player={} client={} err={}",
                                player,
                                client_hash,
                                emsg
                            );
                        } else {
                            tracing::error!(
                                "datagram_recv_error player={} client={} err={}",
                                player,
                                client_hash,
                                emsg
                            );
                        }
                        break;
                    }
                }
            }

            // Handle disconnect / cleanup once loop exits
            if let (Some(callback), Some(player_id), Some(client_id)) =
                (&self.disconnect_callback, &self.player_id, &self.client_id)
            {
                callback(player_id.clone(), client_id.clone());
                if let Some(webhook_receiver) = &self.webhook_receiver {
                    let webhook_receiver_clone = webhook_receiver.clone();
                    let client_id = client_id.clone();
                    let player_name = player_id.clone();
                    tokio::spawn(async move {
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as i64;
                        let presence_packet = QuicNetworkPacket {
                            owner: Some(PacketOwner {
                                name: player_name.clone(),
                                client_id: client_id.clone(),
                            }),
                            packet_type: PacketType::PlayerPresence,
                            data: QuicNetworkPacketData::PlayerPresence(PlayerPresenceEvent {
                                player_name: player_name.clone(),
                                timestamp,
                                event_type: ConnectionEventType::Disconnected,
                            }),
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
