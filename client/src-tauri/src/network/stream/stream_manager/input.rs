use std::{
    sync::Arc,
    thread::sleep,
    time::Duration,
};
use common::structs::packet::QuicNetworkPacket;
use tokio::task::JoinHandle;
use crate::{core::IpcMessage, AudioPacket};
use log::{info, error};

/// The InputStream consumes audio packets from the server
/// Then sends it to the AudioStreamManager::OutputStream
pub(crate) struct InputStream {
    pub bus: Arc<flume::Sender<AudioPacket>>,
    pub rx: spmc::Receiver<IpcMessage>,
    pub tx: spmc::Sender<IpcMessage>,
    pub stream: Option<s2n_quic::stream::ReceiveStream>,
    jobs: Vec<JoinHandle<()>>,
}

impl super::StreamTrait for InputStream {
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

        self.jobs.push(tokio::spawn(async move {
            let mut packet = Vec::<u8>::new();
            while let Ok(Some(data)) = stream.receive().await {
                // Shutdown the stream if we receive a signal
                let message: IpcMessage = rx.recv().unwrap();
                if message.eq(&IpcMessage::Terminate) {
                    info!("Received shutdown signal, stopping network receiving stream.");
                    break;
                }

                // Process the packet, then send it to the AudioStreaManager::OutputStream
                // for playback on the native device
                packet.append(&mut data.to_vec());
                match QuicNetworkPacket::from_stream(&mut packet) {
                    Ok(packets) => {
                        for data in packets {
                            _ = bus.send_async(AudioPacket { data }).await;
                        }
                    },
                    Err(e) => {
                        error!("Couldn't decode packet from recv stream. {:?}", e);
                    }
                }
            }

            drop(stream);
        }));

        Ok(())
    }
}

impl InputStream {
    pub fn new(
        producer: Arc<flume::Sender<AudioPacket>>,
        stream: Option<s2n_quic::stream::ReceiveStream>,
    ) -> Self {
        let (tx, rx) = spmc::channel();
        Self {
            bus: producer.clone(),
            rx,
            tx,
            stream,
            jobs: vec![],
        }
    }
}