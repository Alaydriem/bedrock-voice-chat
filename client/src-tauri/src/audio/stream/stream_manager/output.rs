use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex, mpsc };
use tokio::task::JoinHandle;
use common::structs::audio::AudioDevice;
use crate::AudioPacket;

use super::IpcMessage;
use super::AudioFrame;

pub(crate) struct OutputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Receiver<AudioPacket>>,
    pub rx: spmc::Receiver<IpcMessage>,
    pub tx: spmc::Sender<IpcMessage>,
    pub producer: mpsc::Sender<AudioFrame>,
    pub consumer: mpsc::Receiver<AudioFrame>,
    pub shutdown: Arc<Mutex<AtomicBool>>,
    jobs: Vec<JoinHandle<()>>
}

impl super::StreamTrait for OutputStream {
    fn stop(&mut self) {
        let shutdown = self.shutdown.clone();
        let shutdown = shutdown.lock().unwrap();
        shutdown.store(true, Ordering::Relaxed);
        _ = self.tx.send(IpcMessage::Terminate);
    }

    fn is_stopped(&mut self) -> bool {
        let shutdown = self.shutdown.clone();
        let mut shutdown = shutdown.lock().unwrap();
        return shutdown.get_mut().to_owned();
    }

    fn start(&mut self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

impl OutputStream {
    pub fn new(
        device: Option<AudioDevice>,
        bus: Arc<flume::Receiver<AudioPacket>>
    ) -> Self {
        let (tx, rx) = spmc::channel();
        let (producer, consumer) = mpsc::channel();
        Self {
            device,
            bus,
            rx,
            tx,
            producer,
            consumer,
            shutdown: Arc::new(std::sync::Mutex::new(AtomicBool::new(false))),
            jobs: vec![]
        }
    }
}