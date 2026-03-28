use crate::api::Api;
use crate::audio::types::{AudioDevice, AudioDeviceHost, AudioDeviceType, StreamConfig};
use rodio::cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Wry};
use tauri_plugin_audio_permissions::{AudioPermissionsExt, PermissionRequest, PermissionType};
use tauri_plugin_store::Store;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    store: Arc<Store<Wry>>,
    app_handle: AppHandle,
    input_audio_device: Option<AudioDevice>,
    output_audio_device: AudioDevice,
    pub current_server: Option<String>,
    pub api_client: Option<Api>,
    pub server_pool: Arc<RwLock<HashMap<String, Api>>>,
}

impl AppState {
    pub fn new(store: Arc<Store<Wry>>, app_handle: AppHandle) -> Self {
        let output_device = AppState::setup_audio_device(
            AudioDeviceType::OutputDevice,
            &store,
        ).unwrap_or_else(|e| {
            log::error!("Failed to initialize default output device: {}. Using placeholder.", e);
            AudioDevice {
                io: AudioDeviceType::OutputDevice,
                id: "default".to_string(),
                name: "default".to_string(),
                host: AudioDeviceHost::try_from(cpal::default_host().id())
                    .unwrap_or(AudioDeviceHost::Wasapi),
                stream_configs: vec![],
                display_name: "Default Device".to_string(),
            }
        });

        Self {
            store: store.clone(),
            app_handle,
            // Don't initialize input device at startup - defer until permissions are granted
            // This prevents iOS from prompting for microphone permission on app launch
            input_audio_device: None,
            output_audio_device: output_device,
            current_server: AppState::get_current_server(&store),
            api_client: None,
            server_pool: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the API client with credentials
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
        self.api_client
            .as_ref()
            .ok_or_else(|| "API client not initialized. Please log in first.".to_string())
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
    /// For input devices, this verifies permissions before allowing the change
    pub fn change_audio_device(&mut self, device: AudioDevice) -> Result<(), String> {
        match device.io {
            AudioDeviceType::InputDevice => {
                let request = PermissionRequest {
                    permission_type: PermissionType::Audio,
                };
                let response = self
                    .app_handle
                    .audio_permissions()
                    .check_permission(request)
                    .map_err(|e| format!("Permission check failed: {}", e))?;

                if !response.granted {
                    return Err("Audio permission not granted".to_string());
                }
            }
            _ => {}
        }

        // Create a copy of the device so we can escape certain values
        // @todo!() fix this in the data we get from typescript
        let device: AudioDevice = AudioDevice {
            id: device.id.clone(),
            name: device.name.replace('\"', ""),
            display_name: device.display_name.replace('\"', ""),
            host: device.host,
            io: device.io.clone(),
            stream_configs: device.stream_configs.clone(),
        };

        // Change the stored value
        self.store.set(
            device.io.store_key(),
            json!({
                "id": device.id,
                "name": device.name,
                "type": device.io,
                "config": device.stream_configs,
                "host": device.host,
                "display_name": device.display_name
            }),
        );
        let _ = self.store.save();

        // Update the current state
        match device.io {
            AudioDeviceType::InputDevice => self.input_audio_device = Some(device.clone()),
            AudioDeviceType::OutputDevice => self.output_audio_device = device.clone(),
        }

        Ok(())
    }

    /// Returns the current audio device information for the given device type
    /// For input devices, this lazily initializes the device if permissions are granted
    pub fn get_audio_device(&mut self, io: AudioDeviceType) -> Result<AudioDevice, String> {
        match io {
            AudioDeviceType::InputDevice => {
                // Lazy initialization for input device
                if self.input_audio_device.is_none() {
                    // Check if we have permission (doesn't prompt, just checks)
                    let request = PermissionRequest {
                        permission_type: PermissionType::Audio,
                    };
                    let response = self
                        .app_handle
                        .audio_permissions()
                        .check_permission(request)
                        .map_err(|e| format!("Permission check failed: {}", e))?;

                    if !response.granted {
                        return Err("Audio permission not granted".to_string());
                    }

                    // Permission granted - initialize device now
                    self.input_audio_device = Some(Self::setup_audio_device(
                        AudioDeviceType::InputDevice,
                        &self.store,
                    )?);
                }
                Ok(self.input_audio_device.clone().unwrap())
            }
            AudioDeviceType::OutputDevice => Ok(self.output_audio_device.clone()),
        }
    }

    fn setup_audio_device(io: AudioDeviceType, store: &Arc<Store<Wry>>) -> Result<AudioDevice, String> {
        // Check if stored config exists and has the new `id` field
        let use_stored = match store.get(io.store_key()) {
            Some(s) => {
                if s.get("id").is_none() {
                    log::warn!(
                        "Detected old-format device config for {} (missing 'id' field). Clearing stored config and reverting to system default.",
                        io.store_key()
                    );
                    store.delete(io.store_key());
                    let _ = store.save();
                    false
                } else {
                    true
                }
            }
            None => false,
        };

        let (id, name, host, stream_configs, display_name) = if use_stored {
            let s = store.get(io.store_key()).unwrap();
            (
                s.get("id")
                    .unwrap()
                    .as_str()
                    .unwrap_or("default")
                    .to_string(),
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
            )
        } else {
            let default_host = cpal::default_host();
            let default_device = match io {
                AudioDeviceType::InputDevice => default_host.default_input_device()
                    .ok_or_else(|| "No default input device found. Check your system sound settings.".to_string())?,
                AudioDeviceType::OutputDevice => default_host.default_output_device()
                    .ok_or_else(|| "No default output device found. Check your system sound settings.".to_string())?,
            };

            let default_configs = match io {
                AudioDeviceType::InputDevice => default_device
                    .supported_input_configs()
                    .map_err(|e| format!("Failed to get input configs: {}", e))?
                    .collect(),
                AudioDeviceType::OutputDevice => default_device
                    .supported_output_configs()
                    .map_err(|e| format!("Failed to get output configs: {}", e))?
                    .collect(),
            };

            let device_id = default_device
                .id()
                .map(|id| id.to_string())
                .unwrap_or_else(|_| "default".to_string());

            let device_display_name = default_device
                .description()
                .map(|desc| desc.name().to_string())
                .unwrap_or_else(|_| "Default Device".to_string());

            let stream_config = AudioDevice::to_stream_config(default_configs);

            if stream_config.is_empty() {
                return Err(format!(
                    "INCOMPATIBLE_DEVICE: '{}' has no compatible audio configurations (requires 48kHz or 44.1kHz)",
                    device_display_name
                ));
            }

            (
                device_id,
                "default".to_string(),
                AudioDeviceHost::try_from(default_host.id())
                    .map_err(|_| "Unknown audio host".to_string())?,
                stream_config,
                device_display_name,
            )
        };

        Ok(AudioDevice {
            io,
            id,
            name,
            host,
            stream_configs,
            display_name,
        })
    }

    /// Clears the stored audio device for the given type and re-initializes from OS default
    pub fn clear_audio_device(&mut self, io: AudioDeviceType) -> Result<(), String> {
        self.store.delete(io.store_key());
        let _ = self.store.save();
        let default_device = Self::setup_audio_device(io.clone(), &self.store)?;
        match io {
            AudioDeviceType::InputDevice => self.input_audio_device = Some(default_device),
            AudioDeviceType::OutputDevice => self.output_audio_device = default_device,
        }
        Ok(())
    }

    pub fn get_store(&self) -> Arc<Store<Wry>> {
        self.store.clone()
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
