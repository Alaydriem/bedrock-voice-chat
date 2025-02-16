use crate::{
    events::{
        listeners::listener_trait::ListenerTrait,
        StopAudioDeviceEvent
    },
    structs::app_state::{ AppState, StreamStateType, StreamType }, AudioStreamManager
};
use common::structs::audio::AudioDeviceType;
use log::info;
use std::sync::Mutex;
use tauri::{ App, Event, Manager, Listener };

pub(crate) struct StopAudioDeviceListener<'a> {
    app: &'a mut App
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
        self.app.listen("stop-audio-device", move | event: Event | {
            if let Ok(payload) = serde_json::from_str::<StopAudioDeviceEvent>(&event.payload()) {
                info!("Stopping Audio Device: {}", payload.device.to_string());
                let app_state = handle.state::<Mutex<AppState>>();
                let app_state = app_state.lock().unwrap();
    
                // Stop the current stream by removing it from the app state, stream state configuration
                // The audio threads monitor this on a loop, so if the Arc value ever dissapears
                // Then the thread knows to terminate
                let mut stream_state = app_state.stream_states.lock().unwrap().clone();
                stream_state.stop(
                    StreamStateType::AudioStream,
                    Some(match payload.device {
                        AudioDeviceType::InputDevice => StreamType::InputStream,
                        AudioDeviceType::OutputDevice => StreamType::OutputStream
                    })
                );
                drop(stream_state);
                drop(app_state);

                let audio_stream = handle.state::<Mutex<AudioStreamManager>>();
                let mut audio_stream = audio_stream.lock().unwrap();
                _ = audio_stream.stop(&payload.device);
                drop(audio_stream);
            }
        });
    }
}