use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use common::structs::websocket::PositionSocketEvent;
use tauri::ipc::Channel;
use tokio::sync::{mpsc, oneshot};

use super::PositionSocket;

/// The position sockets the webview has open, by id, so it can close the one it opened.
pub(crate) struct PositionSocketRegistry {
    next_id: AtomicU32,
    stops: Mutex<HashMap<u32, oneshot::Sender<()>>>,
}

impl PositionSocketRegistry {
    const QUEUE_DEPTH: usize = 64;

    pub(crate) fn new() -> Self {
        Self {
            next_id: AtomicU32::new(1),
            stops: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Starts a socket and returns its id. Events arrive on `channel` until the socket ends.
    pub(crate) fn open(
        self: &Arc<Self>,
        url: String,
        protocols: Vec<String>,
        channel: Channel<PositionSocketEvent>,
    ) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (stop_tx, stop_rx) = oneshot::channel();
        if let Ok(mut stops) = self.stops.lock() {
            stops.insert(id, stop_tx);
        }

        let (events_tx, mut events_rx) = mpsc::channel(Self::QUEUE_DEPTH);
        tauri::async_runtime::spawn(PositionSocket::run(url, protocols, events_tx, stop_rx));

        let registry = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            while let Some(event) = events_rx.recv().await {
                if channel.send(event).is_err() {
                    break;
                }
            }
            registry.forget(id);
        });

        id
    }

    pub(crate) fn close(&self, id: u32) {
        if let Some(stop) = self.forget(id) {
            let _ = stop.send(());
        }
    }

    fn forget(&self, id: u32) -> Option<oneshot::Sender<()>> {
        self.stops.lock().ok().and_then(|mut stops| stops.remove(&id))
    }
}
