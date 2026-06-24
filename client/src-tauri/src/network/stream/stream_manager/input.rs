use crate::AudioPacket;
use bytes::Bytes;
use common::s2n_quic::Connection;
use common::structs::packet::{PacketType, QuicNetworkPacket};
use core::{
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
            while let Ok(bytes) = recv_one_datagram(&connection).await {
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
