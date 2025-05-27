use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use common::structs::packet::QuicNetworkPacket;
use tauri::Emitter;
use tokio::task::AbortHandle;
use crate::AudioPacket;
use log::{error, warn};

/// The InputStream consumes audio packets from the server
/// Then sends it to the AudioStreamManager::OutputStream
pub(crate) struct InputStream {
    pub bus: Arc<flume::Sender<AudioPacket>>,
    pub stream: Option<s2n_quic::stream::ReceiveStream>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<moka::future::Cache<String, String>>,
    app_handle: tauri::AppHandle,
}

impl super::StreamTrait for InputStream {
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

    fn is_stopped(&mut self) -> bool {
        self.jobs.len() == 0
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);
        
        let tx = self.bus.clone();
        let mut jobs = vec![];
        let mut stream = self.stream.take().unwrap();

        let shutdown = self.shutdown.clone();
        let app_handle = self.app_handle.clone();
        jobs.push(tokio::spawn(async move {
            log::info!("Started network recv stream.");
            let mut packet = Vec::<u8>::new();
            while let Ok(Some(data)) = stream.receive().await {
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

        _ = app_handle.emit(crate::events::EVENT_NOTIFICATION, crate::events::Notification::new(
            "Network Stream Stopped".to_string(),
            "The input network stream has been stopped.".to_string(),
            Some("warn".to_string()),
            None,
            None,
            None
        ));
        
        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();

        Ok(())
    }
}

impl InputStream {
    pub fn new(
        producer: Arc<flume::Sender<AudioPacket>>,
        stream: Option<s2n_quic::stream::ReceiveStream>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            bus: producer.clone(),
            stream,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::future::Cache::builder().build()),
            app_handle: app_handle.clone()
        }
    }
}