use crate::audio::types::{AudioDevice, AudioDeviceHost, AudioDeviceType, StreamConfig};
use crate::api::Api;
use cpal::traits::{DeviceTrait, HostTrait};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Wry;
use tauri_plugin_store::Store;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    store: Arc<Store<Wry>>,
    input_audio_device: AudioDevice,
    output_audio_device: AudioDevice,
    pub current_server: Option<String>,
    pub api_client: Option<Api>,
    pub server_pool: Arc<RwLock<HashMap<String, Api>>>,
}

impl AppState {
    pub fn new(store: Arc<Store<Wry>>) -> Self {
        Self {
            store: store.clone(),
            input_audio_device: AppState::setup_audio_device(AudioDeviceType::InputDevice, &store),
            output_audio_device: AppState::setup_audio_device(
                AudioDeviceType::OutputDevice,
                &store,
            ),
            current_server: AppState::get_current_server(&store),
            api_client: None,
            server_pool: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the API client with credentials - DUAL MODE
    /// Sets both the default api_client (for backwards compatibility) and adds to server_pool
    pub async fn initialize_api_client(&mut self, endpoint: String, ca_cert: String, pem: String) {
        let api = Api::new(endpoint.clone(), ca_cert, pem);

        // 1. Set as default (legacy - for single server / dashboard)
        self.api_client = Some(api.clone());
        self.current_server = Some(endpoint.clone());

        // 2. Add to pool (new - for multi-server selection)
        let mut pool = self.server_pool.write().await;
        pool.insert(endpoint, api);
    }

    /// Get the API client, returning an error if not initialized
    pub fn get_api_client(&self) -> Result<&Api, String> {
        self.api_client.as_ref().ok_or_else(|| "API client not initialized. Please log in first.".to_string())
    }

    /// Get API client for a specific server from pool
    pub async fn get_api_client_for_server(&self, endpoint: &str) -> Result<Api, String> {
        let pool = self.server_pool.read().await;
        pool.get(endpoint)
            .cloned()
            .ok_or_else(|| format!("No API client initialized for server: {}", endpoint))
    }

    /// Clear the API client (used during logout)
    pub fn clear_api_client(&mut self) {
        self.api_client = None;
    }

    /// Event handler for changing the audio device
    pub fn change_audio_device(&mut self, device: AudioDevice) {
        // Create a copy of the device so we can escape certain values
        // @todo!() fix this in the data we get from typescript
        let device: AudioDevice = AudioDevice {
            name: device.name.replace('\"', ""),
            display_name: device.display_name.replace('\"', ""),
            host: device.host,
            io: device.io.clone(),
            stream_configs: device.stream_configs.clone(),
        };

        // Change the stored value
        self.store.set(
            device.io.to_string(),
            json!({
                "name": device.name,
                "type": device.io,
                "config": device.stream_configs,
                "host": device.host,
                "display_name": device.display_name
            }),
        );

        // Update the current state
        match device.io {
            AudioDeviceType::InputDevice => self.input_audio_device = device.clone(),
            AudioDeviceType::OutputDevice => self.output_audio_device = device.clone(),
        }
    }

    /// Returns the current audio device information for the given device type
    pub fn get_audio_device(&self, io: AudioDeviceType) -> AudioDevice {
        match io {
            AudioDeviceType::InputDevice => self.input_audio_device.clone(),
            AudioDeviceType::OutputDevice => self.output_audio_device.clone(),
        }
    }

    /// Retrieves the current audio device, defaults to `default`,
    /// Which is the system audio driver default
    fn setup_audio_device(io: AudioDeviceType, store: &Arc<Store<Wry>>) -> AudioDevice {
        let (name, host, stream_configs, display_name) = match store.get(io.to_string()) {
            Some(s) => (
                s.get("name").unwrap().to_string().replace('\"', ""),
                serde_json::from_str::<AudioDeviceHost>(
                    s.get("host").unwrap().to_string().as_str(),
                )
                .unwrap(),
                serde_json::from_value::<Vec<StreamConfig>>(s.get("config").unwrap().clone())
                    .unwrap(),
                match s.get("display_name") {
                    Some(name) => name.to_string().replace('\"', ""),
                    None => s.get("name").unwrap().to_string().replace('\"', ""),
                },
            ),
            None => {
                let default_host = cpal::default_host();
                let default_device = match io {
                    AudioDeviceType::InputDevice => default_host.default_input_device().unwrap(),
                    AudioDeviceType::OutputDevice => default_host.default_output_device().unwrap(),
                };

                let default_configs = match io {
                    AudioDeviceType::InputDevice => default_device
                        .supported_input_configs()
                        .unwrap()
                        .map(|s| s)
                        .collect(),
                    AudioDeviceType::OutputDevice => default_device
                        .supported_output_configs()
                        .unwrap()
                        .map(|s| s)
                        .collect(),
                };

                let stream_config = AudioDevice::to_stream_config(default_configs);

                (
                    "default".to_string(),
                    AudioDeviceHost::try_from(default_host.id()).unwrap(),
                    stream_config,
                    default_device.name().unwrap(),
                )
            }
        };

        AudioDevice {
            io,
            name,
            host,
            stream_configs,
            display_name,
        }
    }

    /// Returns the current server from the store, None otherwise
    /// If this returns None, then we should redirect to /servers list
    /// And determin the appropriate server there, and then redirect
    /// back to the login page
    fn get_current_server(store: &Arc<Store<Wry>>) -> Option<String> {
        match store.get("current_server") {
            Some(s) => Some(s.as_str()?.to_string()),
            None => None,
        }
    }
}
