use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use common::structs::packet::QuicNetworkPacket;
use tokio::task::AbortHandle;
use crate::AudioPacket;
use log::{error, info, warn};

/// The InputStream consumes audio packets from the server
/// Then sends it to the AudioStreamManager::OutputStream
pub(crate) struct InputStream {
    pub bus: Arc<flume::Sender<AudioPacket>>,
    pub stream: Option<s2n_quic::stream::ReceiveStream>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    metadata: Arc<moka::sync::Cache<String, String>>
}

impl super::StreamTrait for InputStream {
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
        
        let tx = self.bus.clone();
        let mut jobs = vec![];
        let mut stream = self.stream.take().unwrap();

        let shutdown = self.shutdown.clone();
        jobs.push(tokio::spawn(async move {
            log::info!("Started network recv stream.");
            let mut packet = Vec::<u8>::new();
            while let Ok(Some(data)) = stream.receive().await {
                log::info!("received quic packet.");
                if shutdown.load(Ordering::Relaxed) {
                    warn!("Network stream input handler stopped.");
                    break;
                }

                // Process the packet, then send it to the AudioStreaManager::OutputStream
                // for playback on the native device
                packet.append(&mut data.to_vec());
                match QuicNetworkPacket::from_stream(&mut packet) {
                    Ok(packets) => {
                        for data in packets {
                            _ = tx.send_async(AudioPacket { data }).await;
                        }
                    },
                    Err(e) => {
                        error!("Couldn't decode packet from recv stream. {:?}", e);
                    }
                }
            }

            drop(stream);
        }));

        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();

        Ok(())
    }
}

impl InputStream {
    pub fn new(
        producer: Arc<flume::Sender<AudioPacket>>,
        stream: Option<s2n_quic::stream::ReceiveStream>,
    ) -> Self {
        Self {
            bus: producer.clone(),
            stream,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::sync::Cache::builder().build())
        }
    }
}