use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use common::structs::packet::{DebugPacket, PacketOwner, QuicNetworkPacket};
use tokio::{io::AsyncWriteExt, task::AbortHandle};
use crate::NetworkPacket;
use log::{error, info, warn};

/// The OutputStream consumes PCM NetworkPackets from the AudioStreamManager::InputStream
/// Then sends it to the server
pub(crate) struct OutputStream {
    pub bus: Arc<flume::Receiver<NetworkPacket>>,
    pub packet_owner: Option<PacketOwner>,
    pub stream: Option<s2n_quic::stream::SendStream>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    metadata: Arc<moka::sync::Cache<String, String>>
}

impl super::StreamTrait for OutputStream {
    fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        self.metadata.insert(key, value);
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

    fn is_stopped(&mut self) -> bool {
        self.jobs.len() == 0
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);
        
        let mut jobs = vec![];
        let rx = self.bus.clone();
        let mut stream = self.stream.take().unwrap();
        let packet_owner = self.packet_owner.clone();

        let shutdown = self.shutdown.clone();
        jobs.push(tokio::spawn(async move {
            // Send a DEBUG Packet to initialize the stream on the server
            let debug_packet = QuicNetworkPacket {
                packet_type: common::structs::packet::PacketType::Debug,
                owner: packet_owner.clone(),
                data: common::structs::packet::QuicNetworkPacketData::Debug(
                    DebugPacket(packet_owner.clone().unwrap().name)
                )
            };

            match debug_packet.to_vec() {
                Ok(reader) => {
                    info!("Sent debug packet to server.");
                    _ = stream.write_all(&reader).await;
                },
                Err(e) => {
                    error!("Failed to send DEBUG packet to stream: {:?}", e);
                    return;
                }
            }

            #[allow(irrefutable_let_patterns)]
            while let packet = rx.recv_async().await {
                log::info!("RECEIVED AUDIO PACKET TO SEND TO SERVER");
                match packet {
                    Ok(network_packet) => {
                        if shutdown.load(Ordering::Relaxed) {
                            warn!("Network stream output handler stopped.");
                            break;
                        }

                        let mut packet = network_packet.data;

                        packet.owner = packet_owner.clone();
                        match packet.to_vec() {
                            Ok(reader) => {
                                _ = stream.write_all(&reader).await;
                            }
                            Err(e) => {
                                error!("{}", e.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        error!("QuicNetworkPacket was not valid? {}", e.to_string());
                    }
                }
            }

            _ = stream.close().await;
            drop(stream);
        }));

        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();

        Ok(())
    }
}

impl OutputStream {
    pub fn new(
        consumer: Arc<flume::Receiver<NetworkPacket>>,
        packet_owner: Option<PacketOwner>,
        stream: Option<s2n_quic::stream::SendStream>,
    ) -> Self {
        Self {
            bus: consumer.clone(),
            packet_owner,
            stream,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::sync::Cache::builder().build())
        }
    }
}