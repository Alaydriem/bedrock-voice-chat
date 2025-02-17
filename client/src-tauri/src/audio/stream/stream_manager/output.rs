use crate::AudioPacket;
use common::structs::audio::AudioDevice;
use std::{sync::Arc, thread::sleep, time::Duration};
use tokio::task::JoinHandle;

use crate::core::IpcMessage;

pub(crate) struct OutputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Receiver<AudioPacket>>,
    pub rx: spmc::Receiver<IpcMessage>,
    pub tx: spmc::Sender<IpcMessage>,
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
        Ok(())
    }
}

impl OutputStream {
    pub fn new(device: Option<AudioDevice>, bus: Arc<flume::Receiver<AudioPacket>>) -> Self {
        let (tx, rx) = spmc::channel();
        Self {
            device,
            bus,
            rx,
            tx,
            jobs: vec![],
        }
    }
}
