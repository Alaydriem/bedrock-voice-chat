use crate::{
    events::{
        listeners::listener_trait::ListenerTrait, ChangeAudioDeviceEvent, StopAudioDeviceEvent
    },
    structs::app_state::AppState, AudioStreamManager
};
use log::info;
use std::sync::Mutex;
use tauri::{ App, Event, Manager, Listener, Emitter };

pub(crate) struct ChangeAudioDeviceListener<'a> {
    app: &'a mut App
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
        self.app.listen("change-audio-device", move | event: Event | {
            if let Ok(payload) = serde_json::from_str::<ChangeAudioDeviceEvent>(&event.payload()) {
                info!("Changing audio device event {} {:?}", payload.device.io.to_string(), payload.device);
                
                let device_io = payload.device.clone().io;
                _ = handle.emit("stop-audio-device", StopAudioDeviceEvent { device: device_io.clone() });

                let app_state = handle.state::<Mutex<AppState>>();
                let app_state = app_state.lock().unwrap();

                
                let audio_stream = handle.state::<Mutex<AudioStreamManager>>();
                let mut audio_stream = audio_stream.lock().unwrap();
                audio_stream.init(payload.device.clone());
                _ = audio_stream.restart(&device_io);


                drop(audio_stream);
                drop(app_state);
            }
        });
    }
}