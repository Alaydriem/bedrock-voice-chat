use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::curia;
use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::{RecordType, TSigner};
use tokio::net::UdpSocket;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::config::PlayDnsConfig;

use super::zone::PlayZone;

// Tells the secondaries the zone changed, so they pull it now rather than at the next
// SOA refresh.
//
// Debounced: a burst of changes sends one NOTIFY carrying the last serial. A lost NOTIFY
// costs at most one refresh interval, which is why a secondary that never answers is
// logged and left alone rather than retried for ever.
pub struct PlayZoneNotifier {
    zone: Arc<PlayZone>,
    dns: PlayDnsConfig,
    signer: Option<TSigner>,
    // Taken when the notifier is built, not when its task starts, so a change made in
    // between is still seen.
    changes: watch::Receiver<u32>,
}

impl PlayZoneNotifier {
    // How long each send waits for an acknowledgement before the next one.
    const ACK_WAITS: [Duration; 3] = [
        Duration::from_secs(2),
        Duration::from_secs(4),
        Duration::from_secs(8),
    ];

    pub fn new(zone: Arc<PlayZone>, dns: PlayDnsConfig) -> Self {
        let signer = dns.tsig.as_ref().and_then(|tsig| tsig.signer().ok());

        let changes = zone.subscribe();

        Self {
            zone,
            dns,
            signer,
            changes,
        }
    }

    // With no secondaries the task ends at once: there is nobody to tell.
    pub fn spawn(self, cancel: CancellationToken) -> JoinHandle<()> {
        tokio::spawn(async move {
            if self.dns.secondaries.is_empty() {
                return;
            }

            self.run(cancel).await;
        })
    }

    async fn run(&self, cancel: CancellationToken) {
        let mut changes = self.changes.clone();
        let delay = Duration::from_secs(self.dns.notify_delay_secs);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => return,
                changed = changes.changed() => {
                    if changed.is_err() {
                        return;
                    }
                }
            }

            // Further changes inside the delay restart it.
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(delay) => break,
                    changed = changes.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                }
            }

            let serial = *changes.borrow_and_update();
            // Together, so a secondary that never answers does not hold up the others.
            let attempts = self.dns.secondaries.iter().map(|secondary| async move {
                if !self.notify(secondary.notify, serial).await {
                    curia::warn!("a secondary did not acknowledge NOTIFY", { "secondary": secondary.notify.to_string(), "serial": serial.to_string() });
                }
            });
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = futures_util::future::join_all(attempts) => {}
            }
        }
    }

    // Sends NOTIFY until the secondary acknowledges. False when it never does.
    async fn notify(&self, target: SocketAddr, serial: u32) -> bool {
        let bind = if target.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
        let Ok(socket) = UdpSocket::bind(bind).await else {
            return false;
        };

        let id = Self::message_id();
        let Some(bytes) = self.encode(id, serial) else {
            return false;
        };

        let mut buffer = [0u8; 1024];
        for wait in Self::ACK_WAITS {
            if socket.send_to(&bytes, target).await.is_err() {
                return false;
            }

            let acknowledged = tokio::time::timeout(wait, async {
                loop {
                    let Ok((length, from)) = socket.recv_from(&mut buffer).await else {
                        return false;
                    };
                    if from.ip() == target.ip() && Self::acknowledges(&buffer[..length], id) {
                        return true;
                    }
                }
            })
            .await;

            if acknowledged == Ok(true) {
                return true;
            }
        }

        false
    }

    fn encode(&self, id: u16, serial: u32) -> Option<Vec<u8>> {
        let mut message = Message::new(id, MessageType::Query, OpCode::Notify);
        message.metadata.authoritative = true;

        let mut query = Query::new();
        query
            .set_name(self.zone.origin().clone())
            .set_query_type(RecordType::SOA);
        message.add_query(query);
        message.add_answer(self.zone.soa_record_at(serial));

        if let Some(signer) = &self.signer {
            message.finalize(signer, Self::now()).ok()?;
        }

        message.to_vec().ok()
    }

    fn acknowledges(bytes: &[u8], id: u16) -> bool {
        Message::from_vec(bytes).is_ok_and(|reply| {
            reply.metadata.message_type == MessageType::Response
                && reply.metadata.op_code == OpCode::Notify
                && reply.metadata.id == id
        })
    }

    fn message_id() -> u16 {
        let mut bytes = [0u8; 2];
        getrandom::fill(&mut bytes).expect("the system random source is available");
        u16::from_le_bytes(bytes)
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default()
    }
}
