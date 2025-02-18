use std::{
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread::sleep,
    time::Duration,
};
use common::structs::packet::QuicNetworkPacket;
use tokio::task::JoinHandle;
use crate::{core::IpcMessage, AudioPacket};
use log::{error, warn};

/// The InputStream consumes audio packets from the server
/// Then sends it to the AudioStreamManager::OutputStream
pub(crate) struct InputStream {
    pub bus: Arc<flume::Sender<AudioPacket>>,
    pub rx: spmc::Receiver<IpcMessage>,
    pub tx: spmc::Sender<IpcMessage>,
    pub stream: Option<s2n_quic::stream::ReceiveStream>,
    jobs: Vec<JoinHandle<()>>,
    shutdown: Arc<Mutex<AtomicBool>>,
    metadata: Arc<moka::sync::Cache<String, String>>
}

impl super::StreamTrait for InputStream {
    fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        self.metadata.insert(key, value);
        Ok(())
    }

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

        let monitor_shutdown = self.shutdown.clone();
        self.jobs.push(tokio::spawn(async move {
            match monitor_shutdown.lock() {
                Ok(shutdown) => match rx.recv() {
                    Ok(result) => match result {
                        IpcMessage::Terminate => shutdown.store(true, Ordering::Relaxed),
                    },
                    Err(e) => {
                        warn!("{:?}", e);
                    }
                },
                Err(e) => {
                    warn!("{:?}", e);
                }
            };
        }));

        let shutdown = self.shutdown.clone();
        self.jobs.push(tokio::spawn(async move {
            let mut packet = Vec::<u8>::new();
            while let Ok(Some(data)) = stream.receive().await {
                match shutdown.lock() {
                    Ok(mut shutdown) => {
                        if shutdown.get_mut().to_owned() {
                            break;
                        }
                    },
                    Err(_) => {}
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
            shutdown: Arc::new(std::sync::Mutex::new(AtomicBool::new(false))),
            metadata: Arc::new(moka::sync::Cache::builder().build())
        }
    }
}