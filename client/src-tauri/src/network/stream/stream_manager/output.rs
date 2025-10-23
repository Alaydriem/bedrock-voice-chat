use crate::NetworkPacket;
use bytes::Bytes;
use common::structs::packet::{DebugPacket, PacketOwner, QuicNetworkPacket};
use log::{error, info, warn};
use s2n_quic::Connection;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::Emitter;
use tokio::{task::AbortHandle, time::Instant};

use common::consts::version::PROTOCOL_VERSION as CLIENT_VERSION;

/// The OutputStream consumes PCM NetworkPackets from the AudioStreamManager::InputStream
/// Then sends it to the server
pub(crate) struct OutputStream {
    pub bus: Arc<flume::Receiver<NetworkPacket>>,
    pub packet_owner: Option<PacketOwner>,
    pub connection: Option<Arc<Connection>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<moka::future::Cache<String, String>>,
    app_handle: tauri::AppHandle,
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

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);

        let mut jobs = vec![];
        let rx = self.bus.clone();
        let connection = self.connection.clone().unwrap();
        let packet_owner = self.packet_owner.clone();
        let app_handle = self.app_handle.clone();

        let shutdown = self.shutdown.clone();
        jobs.push(tokio::spawn(async move {

            // Send a DEBUG Packet to initialize the stream on the server
            let debug_packet = QuicNetworkPacket {
                packet_type: common::structs::packet::PacketType::Debug,
                owner: packet_owner.clone(),
                data: common::structs::packet::QuicNetworkPacketData::Debug(
                    DebugPacket {
                        owner: packet_owner.clone().unwrap().name,
                        version: CLIENT_VERSION.to_string(),
                        timestamp: Instant::now().elapsed().as_millis() as u64,
                    }
                )
            };

            match debug_packet.to_datagram() {
                Ok(bytes) => {
                    info!("Sent debug packet to server.");
                    let payload = Bytes::from(bytes);
                    if let Err(e) = connection.datagram_mut(|dg: &mut s2n_quic::provider::datagram::default::Sender| dg.send_datagram(payload.clone())) { error!("Debug datagram send error: {:?}", e); }
                }
                Err(e) => { error!("Failed to serialize DEBUG packet: {:?}", e); }
            }

            let mut error_count = 0;
            #[allow(irrefutable_let_patterns)]
            while let packet = rx.recv_async().await {
                match packet {
                    Ok(network_packet) => {
                        if shutdown.load(Ordering::Relaxed) {
                            warn!("Network stream output handler stopped.");
                            break;
                        }

                        let mut quic_network_packet = network_packet.data;
                        quic_network_packet.owner = packet_owner.clone();

                        // Send immediately for real-time performance
                        match quic_network_packet.to_datagram() {
                            Ok(bytes) => {
                                let payload = Bytes::from(bytes);
                                let send_res = connection.datagram_mut(|dg: &mut s2n_quic::provider::datagram::default::Sender| dg.send_datagram(payload.clone()));
                                if let Err(e) = send_res {
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

            _ = app_handle.emit(crate::events::event::notification::EVENT_NOTIFICATION, crate::events::event::notification::Notification::new(
                "Network Stream Stopped".to_string(),
                "The output network stream has been stopped.".to_string(),
                Some("warn".to_string()),
                None,
                None,
                None
            ));

            // No stream close; connection closed elsewhere if needed
        }));

        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();

        Ok(())
    }
}

impl OutputStream {
    pub fn new(
        consumer: Arc<flume::Receiver<NetworkPacket>>,
        packet_owner: Option<PacketOwner>,
        connection: Option<Arc<Connection>>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            bus: consumer.clone(),
            packet_owner,
            connection,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::future::Cache::builder().build()),
            app_handle: app_handle.clone(),
        }
    }
}
