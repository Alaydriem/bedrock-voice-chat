use crate::{
    audio::events::StopAudioDeviceEvent, events::ListenerTrait, AudioStreamManager
};
use log::info;
use std::sync::Mutex;
use tauri::{App, Event, Listener, Manager};

pub(crate) struct StopAudioDeviceListener<'a> {
    app: &'a mut App,
}

impl<'a> StopAudioDeviceListener<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }

    pub fn listen(&self) {
        self.listener();
    }
}

impl<'a> ListenerTrait for StopAudioDeviceListener<'a> {
    fn listener(&self) {
        let handle = self.app.handle().clone();
        self.app.listen("stop-audio-device", move |event: Event| {
            if let Ok(payload) = serde_json::from_str::<StopAudioDeviceEvent>(&event.payload()) {
                info!("Stopping Audio Device: {}", payload.device.to_string());
                let audio_stream = handle.state::<Mutex<AudioStreamManager>>();
                let mut audio_stream = audio_stream.lock().unwrap();
                _ = audio_stream.stop(&payload.device);
                drop(audio_stream);
            }
        });
    }
}
