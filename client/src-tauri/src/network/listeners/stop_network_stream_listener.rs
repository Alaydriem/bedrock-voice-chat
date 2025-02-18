use crate::{
    events::ListenerTrait,
    network::events::StopNetworkStreamEvent,
    NetworkStreamManager
};
use log::info;
use std::sync::Mutex;
use tauri::{App, Event, Listener, Manager};

pub(crate) struct StopNetworkStreamListener<'a> {
    app: &'a mut App,
}

impl<'a> StopNetworkStreamListener<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub fn listen(&self) {
        self.listener();
    }
}

impl<'a> ListenerTrait for StopNetworkStreamListener<'a> {
    fn listener(&self) {
        let handle = self.app.handle().clone();
        self.app.listen("stop-network-stream", move |event: Event| {
            if let Ok(_) = serde_json::from_str::<StopNetworkStreamEvent>(&event.payload()) {
                info!("Stopping Network Stream");
                let network_stream = handle.state::<Mutex<NetworkStreamManager>>();
                let mut network_stream = network_stream.lock().unwrap();
                _ = network_stream.stop();
            }
        });
    }
}
