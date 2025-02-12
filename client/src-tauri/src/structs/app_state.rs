use common::structs::audio::{ AudioDevice, AudioDeviceType, StreamConfig};
use std::sync::Arc;
use tauri_plugin_store::Store;
use tauri::Wry;
use serde_json::json;
use cpal::traits::{DeviceTrait, HostTrait};

#[derive(Clone)]
pub struct AppState {
    store: Arc<Store<Wry>>,
    input_audio_device: AudioDevice,
    output_audio_device: AudioDevice,
    pub current_server: Option<String>
}

impl AppState {
    /// Initializes the Application State struct
    pub fn new(store: Arc<Store<Wry>>) -> Self {
        Self {
            store: store.clone(),
            input_audio_device: AppState::setup_audio_device(AudioDeviceType::InputDevice, &store),
            output_audio_device: AppState::setup_audio_device(AudioDeviceType::OutputDevice, &store),
            current_server: AppState::get_current_server(&store)
        }
    }

    /// Event handler for changing the audio device
    pub fn change_audio_device(&mut self, io: &AudioDevice) {
        // Change the stored value
        self.store.set(io.io.to_string(), json!({
            "name": io.name,
            "type": io.io.to_string(),
            "config": io.stream_configs,
            "host": io.host,
            "display_name": io.display_name
        }));

        // Update the current state
        match io.io {
            AudioDeviceType::InputDevice => self.input_audio_device = io.clone(),
            AudioDeviceType::OutputDevice => self.output_audio_device = io.clone()
        }
    }

    /// Returns the current audio device information for the given device type
    pub fn get_audio_device(&self, io: AudioDeviceType) -> AudioDevice {
        match io {
            AudioDeviceType::InputDevice => self.input_audio_device.clone(),
            AudioDeviceType::OutputDevice => self.output_audio_device.clone()
        }
    }

    /// Retrieves the current audio device, defaults to `default`,
    /// Which is the system audio driver default
    fn setup_audio_device(io: AudioDeviceType, store: &Arc<Store<Wry>>) -> AudioDevice {
        let (name, host, stream_configs, display_name) = match store.get(io.to_string()) {
            Some(s) => (
                s.get("name").unwrap().to_string(),
                s.get("host").unwrap().to_string(),
                serde_json::from_value::<Vec<StreamConfig>>(s.get("config").unwrap().clone()).unwrap(),
                match s.get("display_name") {
                    Some(name) => name.to_string(),
                    None => s.get("name").unwrap().to_string()
                }
            )
            ,
            None => {
                let default_host = cpal::default_host();
                let default_device = match io {
                    AudioDeviceType::InputDevice => default_host.default_input_device().unwrap(),
                    AudioDeviceType::OutputDevice => default_host.default_output_device().unwrap()
                };

                let default_configs = match io {
                    AudioDeviceType::InputDevice => default_device.supported_input_configs().unwrap().map(|s| s).collect(),
                    AudioDeviceType::OutputDevice => default_device.supported_output_configs().unwrap().map(|s| s).collect()
                };

                let stream_config = AudioDevice::to_stream_config(default_configs);

                (
                    "default".to_string(),
                    default_host.id().name().to_string(),
                    stream_config,
                    default_device.name().unwrap()
                )
            }
        };

        AudioDevice {
            io,
            name,
            host,
            stream_configs,
            display_name
        }
    }

    /// Returns the current server from the store, None otherwise
    /// If this returns None, then we should redirect to /servers list
    /// And determin the appropriate server there, and then redirect
    /// back to the login page
    fn get_current_server(store: &Arc<Store<Wry>>) -> Option<String> {
        match store.get("current_server") {
            Some(s) => Some(s.get("value").unwrap().to_string()),
            None => None
        }
    }
}