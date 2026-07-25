use crate::AudioPacket;
use bytes::Bytes;
use common::s2n_quic::Connection;
use common::s2n_quic::provider::datagram::default::DatagramError;
use common::structs::network::QuicCloseCode;
use common::structs::packet::{PacketType, QuicNetworkPacket};
use core::{
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use log::{error, info};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tokio::task::AbortHandle;

use super::HealthMonitorState;

const DECODE_ERROR_LOG_INTERVAL: Duration = Duration::from_secs(30);

// Datagrams are TLS-integrity-protected, so a decode failure is never corruption,
// it is a wire-format mismatch. This many in a row with zero successful decodes in
// between means the peer speaks an incompatible protocol and the connection is dead.
const DECODE_ERROR_FATAL_THRESHOLD: u32 = 10;

/// The InputStream consumes audio packets from the server
/// Then sends it to the AudioStreamManager::OutputStream
pub(crate) struct InputStream {
    pub bus: Arc<flume::Sender<AudioPacket>>,
    pub connection: Option<Arc<Connection>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<moka::future::Cache<String, String>>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
    pub health_state: Arc<HealthMonitorState>,
}

impl common::traits::StreamTrait for InputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        let metadata = self.metadata.clone();
        metadata.insert(key, value).await;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(true, Ordering::Relaxed);
        for job in &self.jobs {
            job.abort();
        }

        self.jobs = vec![];
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.jobs.len() == 0
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);

        let tx = self.bus.clone();
        let mut jobs = vec![];
        let connection = self.connection.clone().unwrap();

        let shutdown = self.shutdown.clone();
        let health_state = self.health_state.clone();
        jobs.push(tokio::spawn(async move {
            log::info!("Started network recv stream.");
            let mut decode_errors: u64 = 0;
            let mut consecutive_decode_errors: u32 = 0;
            let mut last_decode_error_log: Option<Instant> = None;
            while let Some(bytes) = Self::recv_next(&connection, &health_state).await {
                if shutdown.load(Ordering::Relaxed) {
                    info!("Network stream input handler stopped.");
                    break;
                }
                match QuicNetworkPacket::from_datagram(&bytes) {
                    Ok(packet) => {
                        consecutive_decode_errors = 0;
                        health_state.on_packet_received();

                        if packet.packet_type == PacketType::HealthCheck {
                            health_state.on_health_check_received();
                            log::trace!("Received health check response from server");
                            continue;
                        }

                        #[cfg(feature = "e2e")]
                        if packet.packet_type == PacketType::AudioFrame {
                            crate::testkit::counters::TransportCounters::increment_from_quic();
                        }
                        _ = tx.send_async(AudioPacket { data: packet }).await;
                    }
                    Err(e) => {
                        decode_errors += 1;
                        consecutive_decode_errors += 1;
                        let should_log = last_decode_error_log
                            .map(|t| t.elapsed() >= DECODE_ERROR_LOG_INTERVAL)
                            .unwrap_or(true);
                        if should_log {
                            // A sustained run of these almost always means a client/server protocol mismatch
                            error!(
                                "Couldn't decode datagram packet ({} failure(s) in the last interval, likely a client/server version mismatch). {:?}",
                                decode_errors, e
                            );
                            decode_errors = 0;
                            last_decode_error_log = Some(Instant::now());
                        }

                        if consecutive_decode_errors >= DECODE_ERROR_FATAL_THRESHOLD {
                            error!(
                                "Aborting connection: {} consecutive undecodable datagrams indicate an incompatible server protocol.",
                                consecutive_decode_errors
                            );
                            health_state.signal_protocol_error();
                            shutdown.store(true, Ordering::SeqCst);
                            break;
                        }
                    }
                }
            }
        }));

        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();

        Ok(())
    }
}

impl InputStream {
    /// Receive the next datagram, or `None` when the recv stream has ended. A server
    /// refusal of this connection's identity is signalled to the health monitor
    /// before ending so the reconnect loop stops instead of re-dialing.
    async fn recv_next(connection: &Connection, health_state: &HealthMonitorState) -> Option<Bytes> {
        match recv_one_datagram(connection).await {
            Ok(bytes) => Some(bytes),
            Err(failure) => {
                if Self::is_refused(&failure, connection) {
                    error!(
                        "Server refused this connection's identity; not retrying. Sign in again if your credentials were revoked."
                    );
                    health_state.signal_unauthorized();
                } else {
                    info!("Network recv stream ended: {}", failure);
                }
                None
            }
        }
    }

    /// Whether the server refused this connection's identity.
    ///
    /// The close code surfaces on two different places depending on timing: the
    /// datagram error carries it when a receive was in flight as the close landed,
    /// but a refusal issued at `accept()` arrives before the first poll, and is then
    /// only visible by querying the connection handle. Both are checked, because the
    /// server refuses before the client has sent anything.
    fn is_refused(failure: &RecvFailure, connection: &Connection) -> bool {
        if let RecvFailure::Datagram(DatagramError::ConnectionError { error, .. }) = failure {
            if Self::is_unauthorized_close(error) {
                return true;
            }
        }

        match connection.application_protocol() {
            Err(e) => Self::is_unauthorized_close(&e),
            Ok(_) => false,
        }
    }

    /// True when the peer closed with our Unauthorized application error code, which
    /// means the server rejected this connection's mTLS identity.
    fn is_unauthorized_close(error: &common::s2n_quic::connection::Error) -> bool {
        match error {
            common::s2n_quic::connection::Error::Application { error, .. } => {
                QuicCloseCode::from_u64(u64::from(*error)) == Some(QuicCloseCode::Unauthorized)
            }
            _ => false,
        }
    }

    pub fn new(
        producer: Arc<flume::Sender<AudioPacket>>,
        connection: Option<Arc<Connection>>,
        app_handle: tauri::AppHandle,
        health_state: Arc<HealthMonitorState>,
    ) -> Self {
        Self {
            bus: producer.clone(),
            connection,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::future::Cache::builder().build()),
            app_handle: app_handle.clone(),
            health_state,
        }
    }
}

// Why a datagram receive ended. The concrete `DatagramError` is preserved rather
// than flattened into `anyhow`, because a server-initiated close carries an
// application error code that only the typed value exposes — every connection-level
// failure renders as the same opaque string through `Display`.
pub(crate) enum RecvFailure {
    /// The connection itself failed or was closed by the peer.
    Datagram(DatagramError),
    /// The datagram provider could not be queried, i.e. the connection is already gone.
    Query(String),
}

impl fmt::Display for RecvFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecvFailure::Datagram(e) => write!(f, "{}", e),
            RecvFailure::Query(e) => write!(f, "{}", e),
        }
    }
}

struct RecvDatagram<'c> {
    conn: &'c Connection,
}
impl<'c> RecvDatagram<'c> {
    fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }
}
impl<'c> Future for RecvDatagram<'c> {
    type Output = Result<Bytes, RecvFailure>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.conn.datagram_mut(
            |r: &mut common::s2n_quic::provider::datagram::default::Receiver| {
                r.poll_recv_datagram(cx)
            },
        ) {
            Ok(Poll::Ready(Ok(bytes))) => Poll::Ready(Ok(bytes)),
            Ok(Poll::Ready(Err(e))) => Poll::Ready(Err(RecvFailure::Datagram(e))),
            Ok(Poll::Pending) => Poll::Pending,
            Err(e) => Poll::Ready(Err(RecvFailure::Query(e.to_string()))),
        }
    }
}
async fn recv_one_datagram(conn: &Connection) -> Result<Bytes, RecvFailure> {
    RecvDatagram::new(conn).await
}
