use std::sync::Arc;

// Seeds the store so AppState construction and the audio device setup never
// touch real Cpal devices. The Fake backend bypasses device enumeration, but
// AppState still reads these keys during construction.
pub struct StoreSeeder;

impl StoreSeeder {
    fn fake_device(io: &str) -> serde_json::Value {
        let channels = if io == "input_audio_device" { 1 } else { 2 };
        serde_json::json!({
            "id": "fake",
            "name": "fake",
            "host": serde_json::to_value(common::structs::audio::AudioDeviceHost::default())
                .unwrap_or(serde_json::Value::Null),
            "config": [{
                "channels": channels,
                "sample_rate": 48_000,
                "sample_format": "f32",
                "buffer_size_min": 0,
                "buffer_size_max": 4096
            }],
            "display_name": "Fake Device"
        })
    }

    pub fn seed(store: &Arc<tauri_plugin_store::Store<tauri::Wry>>) {
        store.set("current_player", serde_json::json!("E2ePlayer"));
        store.set("input_audio_device", Self::fake_device("input_audio_device"));
        store.set("output_audio_device", Self::fake_device("output_audio_device"));
        store.set(
            "install_id",
            serde_json::json!("00000000-0000-0000-0000-000000000000"),
        );
        store.set("use_noise_gate", serde_json::json!(false));
        let _ = store.save();
    }
}
