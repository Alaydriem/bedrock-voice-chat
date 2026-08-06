use crate::NetworkPacket;
use bytes::Bytes;
use common::s2n_quic::Connection;
use common::structs::packet::{DebugPacket, QuicNetworkPacket};
use log::{error, info};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::Emitter;
use tokio::{task::AbortHandle, time::Instant};

use common::consts::version::PROTOCOL_VERSION as CLIENT_VERSION;

/// The OutputStream consumes PCM NetworkPackets from the AudioStreamManager::InputStream
/// Then sends it to the server
pub(crate) struct OutputStream {
    pub bus: Arc<flume::Receiver<NetworkPacket>>,
    /// This connection's canonical identity. Reported in the opening Debug packet only —
    /// every other packet leaves here unattributed, because the server takes the sender
    /// from the certificate rather than from anything written here.
    pub identity: String,
    pub connection: Option<Arc<Connection>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<moka::future::Cache<String, String>>,
    app_handle: tauri::AppHandle,
    transport_stats: Arc<crate::diagnostics::TransportStats>,
}

impl common::traits::StreamTrait for OutputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        let metadata = self.metadata.clone();
        metadata.insert(key, value).await;

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(true, Ordering::Relaxed);

        // Then hard terminate them
        for job in &self.jobs {
            job.abort();
        }

        self.jobs = vec![];
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.jobs.len() == 0
    }

    #[tracing::instrument(skip(self))]
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);

        let mut jobs = vec![];
        let rx = self.bus.clone();
        let connection = self.connection.clone().unwrap();
        let identity = self.identity.clone();
        let app_handle = self.app_handle.clone();
        let transport_stats = self.transport_stats.clone();

        let shutdown = self.shutdown.clone();
        jobs.push(tokio::spawn(async move {

            // Send a DEBUG Packet to initialize the stream on the server
            let debug_packet = QuicNetworkPacket {
                packet_type: common::structs::packet::PacketType::Debug,
                data: common::structs::packet::QuicNetworkPacketData::Debug(
                    DebugPacket {
                        identity: identity.clone(),
                        version: CLIENT_VERSION.to_string(),
                        timestamp: Instant::now().elapsed().as_millis() as u64,
                    }
                ),
                // Client-to-server, not a server fan-out, so this envelope carries no sequence.
                ..Default::default()
            };

            match debug_packet.to_datagram() {
                Ok(bytes) => {
                    info!("Sent debug packet to server.");
                    let payload = Bytes::from(bytes);
                    if let Err(e) = connection.datagram_mut(|dg: &mut common::s2n_quic::provider::datagram::default::Sender| dg.send_datagram(payload.clone())) { error!("Debug datagram send error: {:?}", e); }
                }
                Err(e) => { error!("Failed to serialize DEBUG packet: {:?}", e); }
            }

            let mut error_count = 0;
            #[allow(irrefutable_let_patterns)]
            while let packet = rx.recv_async().await {
                match packet {
                    Ok(network_packet) => {
                        if shutdown.load(Ordering::Relaxed) {
                            info!("Network stream output handler stopped.");
                            break;
                        }

                        let quic_network_packet = network_packet.data;

                        // Send immediately for real-time performance
                        match quic_network_packet.to_datagram() {
                            Ok(bytes) => {
                                let payload = Bytes::from(bytes);
                                let send_res = connection.datagram_mut(|dg: &mut common::s2n_quic::provider::datagram::default::Sender| dg.send_datagram(payload.clone()));
                                if let Err(e) = send_res {
                                    transport_stats.record_send_error();
                                    error_count += 1;
                                    if error_count == 100 {
                                        _ = app_handle.emit(crate::events::event::notification::EVENT_NOTIFICATION, crate::events::event::notification::Notification::new(
                                            "High Network Datagram Errors!".to_string(),
                                            "BVC is currently having difficulties connecting to the server. Audio packets may be delayed or out of sync. A restart is recommended.".to_string(),
                                            Some("error".to_string()),
                                            Some(e.to_string()),
                                            None,
                                            None
                                        ));
                                    }
                                } else {
                                    error_count = 0;
                                    transport_stats.record_sent();
                                    if quic_network_packet.packet_type
                                        == common::structs::packet::PacketType::AudioFrame
                                    {
                                        transport_stats.record_frame_sent();
                                    }
                                }
                            }
                            Err(e) => { error!("{}", e.to_string()); }
                        }
                    }
                    Err(e) => {
                        error!("QuicNetworkPacket was not valid? {}", e.to_string());
                    }
                }
            }

            // No stream close; connection closed elsewhere if needed
        }));

        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();

        Ok(())
    }
}

impl OutputStream {
    pub fn new(
        consumer: Arc<flume::Receiver<NetworkPacket>>,
        identity: String,
        connection: Option<Arc<Connection>>,
        app_handle: tauri::AppHandle,
        transport_stats: Arc<crate::diagnostics::TransportStats>,
    ) -> Self {
        Self {
            bus: consumer.clone(),
            identity,
            connection,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::future::Cache::builder().build()),
            app_handle: app_handle.clone(),
            transport_stats,
        }
    }
}
