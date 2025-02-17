use crate::{
    events::ListenerTrait,
    network::events::ChangeNetworkStreamEvent,
};
use log::info;
use tauri::{App, Event, Listener};

pub(crate) struct ChangeNetworkStreamListener<'a> {
    app: &'a mut App,
}

impl<'a> ChangeNetworkStreamListener<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub fn listen(&self) {
        self.listener();
    }
}

impl<'a> ListenerTrait for ChangeNetworkStreamListener<'a> {
    fn listener(&self) {
        self.app.listen("change-network-stream", move |event: Event| {
            if let Ok(_) = serde_json::from_str::<ChangeNetworkStreamEvent>(&event.payload()) {
                info!("Changing Network Stream");
                // This doesn't do anything, it's exclusively to implement the event pattern
            }
        });
    }
}
