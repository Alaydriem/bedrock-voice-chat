use crate::{
    audio::events::ChangeAudioDeviceEvent,
    audio::events::StopAudioDeviceEvent,
     events::ListenerTrait
};
use log::info;
use tauri::{App, Emitter, Event, Listener};

pub(crate) struct ChangeAudioDeviceListener<'a> {
    app: &'a mut App,
}

impl<'a> ChangeAudioDeviceListener<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub fn listen(&self) {
        self.listener();
    }
}

impl<'a> ListenerTrait for ChangeAudioDeviceListener<'a> {
    fn listener(&self) {
        let handle = self.app.handle().clone();
        self.app.listen("change-audio-device", move |event: Event| {
            info!("event received {:?}", event.clone());
            if let Ok(payload) = serde_json::from_str::<ChangeAudioDeviceEvent>(&event.payload()) {
                info!("event processed");
                info!(
                    "Changing audio device event {} {:?}",
                    payload.device.io.to_string(),
                    payload.device
                );

                let device_io = payload.device.clone().io;
                _ = handle.emit(
                    "stop-audio-device",
                    StopAudioDeviceEvent {
                        device: device_io.clone(),
                    },
                );                
            }
        });
    }
}
