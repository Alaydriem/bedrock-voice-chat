use std::{
    sync::Arc,
    thread::sleep,
    time::Duration,
};
use common::structs::packet::{DebugPacket, PacketOwner, QuicNetworkPacket};
use tokio::{io::AsyncWriteExt, task::JoinHandle};
use crate::{core::IpcMessage, NetworkPacket};
use log::{info, error};

/// The OutputStream consumes PCM NetworkPackets from the AudioStreamManager::InputStream
/// Then sends it to the server
pub(crate) struct OutputStream {
    pub bus: Arc<flume::Receiver<NetworkPacket>>,
    pub rx: spmc::Receiver<IpcMessage>,
    pub tx: spmc::Sender<IpcMessage>,
    pub packet_owner: Option<PacketOwner>,
    pub stream: Option<s2n_quic::stream::SendStream>,
    jobs: Vec<JoinHandle<()>>,
}

impl super::StreamTrait for OutputStream {
    fn stop(&mut self) {
        _ = self.tx.send(IpcMessage::Terminate);

        // Give the threads time to detect that they should gracefully shut down
        _ = sleep(Duration::from_secs(1));

        // Then hard terminate them
        for job in &self.jobs {
            job.abort();
        }

        self.jobs = vec![];
    }

    fn is_stopped(&mut self) -> bool {
        self.jobs.len() == 0
    }

    fn start(&mut self) -> Result<(), anyhow::Error> {
        let bus = self.bus.clone();
        let rx = self.rx.clone();
        let mut stream = self.stream.take().unwrap();
        let packet_owner = self.packet_owner.clone();
        self.jobs.push(tokio::spawn(async move {
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
                    _ = stream.write_all(&reader).await;
                },
                Err(e) => {
                    error!("Failed to send DEBUG packet to stream: {:?}", e);
                    return;
                }
            }

            #[allow(irrefutable_let_patterns)]
            while let packet = bus.recv_async().await {
                match packet {
                    Ok(network_packet) => {
                        // Shutdown the stream if we receive a signal
                        let message: IpcMessage = rx.recv().unwrap();
                        if message.eq(&IpcMessage::Terminate) {
                            info!("Received shutdown signal, stopping network receiving stream.");
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

        Ok(())
    }
}

impl OutputStream {
    pub fn new(
        consumer: Arc<flume::Receiver<NetworkPacket>>,
        packet_owner: Option<PacketOwner>,
        stream: Option<s2n_quic::stream::SendStream>,
    ) -> Self {
        let (tx, rx) = spmc::channel();
        Self {
            bus: consumer.clone(),
            rx,
            tx,
            packet_owner,
            stream,
            jobs: vec![],
        }
    }
}