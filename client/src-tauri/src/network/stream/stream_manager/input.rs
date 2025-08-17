use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use common::structs::packet::QuicNetworkPacket;
use bytes::Bytes;
use core::{future::Future, pin::Pin, task::{Context, Poll}};
use s2n_quic::Connection;
use tauri::Emitter;
use tokio::task::AbortHandle;
use crate::AudioPacket;
use log::{error, warn};

/// The InputStream consumes audio packets from the server
/// Then sends it to the AudioStreamManager::OutputStream
pub(crate) struct InputStream {
    pub bus: Arc<flume::Sender<AudioPacket>>,
    pub connection: Option<Arc<Connection>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<moka::future::Cache<String, String>>,
    app_handle: tauri::AppHandle,
}

impl common::traits::StreamTrait for InputStream {
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
        
        let tx = self.bus.clone();
        let mut jobs = vec![];
    let connection = self.connection.clone().unwrap();

        let shutdown = self.shutdown.clone();
        let app_handle = self.app_handle.clone();
        jobs.push(tokio::spawn(async move {
            log::info!("Started network recv stream.");
            while let Ok(bytes) = recv_one_datagram(&connection).await {
                if shutdown.load(Ordering::Relaxed) {
                    warn!("Network stream input handler stopped.");
                    break;
                }
                match QuicNetworkPacket::from_datagram(&bytes) {
                    Ok(packet) => { _ = tx.send_async(AudioPacket { data: packet }).await; }
                    Err(e) => { error!("Couldn't decode datagram packet. {:?}", e); }
                }
            }
        }));

        _ = app_handle.emit(crate::events::event::notification::EVENT_NOTIFICATION, crate::events::event::notification::Notification::new(
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
        connection: Option<Arc<Connection>>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            bus: producer.clone(),
            connection,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::future::Cache::builder().build()),
            app_handle: app_handle.clone()
        }
    }
}

// Minimal datagram future for client
struct RecvDatagram<'c> { conn: &'c Connection }
impl<'c> RecvDatagram<'c> { fn new(conn: &'c Connection) -> Self { Self { conn } } }
impl<'c> Future for RecvDatagram<'c> {
    type Output = Result<Bytes, anyhow::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.conn.datagram_mut(|r: &mut s2n_quic::provider::datagram::default::Receiver| r.poll_recv_datagram(cx)) {
            Ok(Poll::Ready(Ok(bytes))) => Poll::Ready(Ok(bytes)),
            Ok(Poll::Ready(Err(e))) => Poll::Ready(Err(anyhow::anyhow!(e))),
            Ok(Poll::Pending) => Poll::Pending,
            Err(e) => Poll::Ready(Err(anyhow::anyhow!(e)))
        }
    }
}
async fn recv_one_datagram(conn: &Connection) -> Result<Bytes, anyhow::Error> { RecvDatagram::new(conn).await }