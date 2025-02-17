use audio::AudioPacket;
use blake2::{Blake2s256, Digest};
use common::ncryptflib::rocket::base64;
use flume::{Receiver, Sender};
use network::NetworkPacket;
use serde_json::json;
use std::{
    fs::File,
    sync::{Arc, Mutex},
};
use async_mutex::Mutex as AsyncMutex;
use tauri::path::BaseDirectory;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

use audio::AudioStreamManager;
use network::NetworkStreamManager;

mod audio;
mod auth;
mod commands;
mod core; 
mod events;
mod network;
mod structs;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // For desktop applications, enforce only a single running instance at a time
    // And enable logging
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }));

        builder = builder.plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
    }

    builder
        //.plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            // Authentication
            crate::auth::commands::server_login,
            // Environment Variable Data
            crate::commands::env::get_env,
            crate::commands::env::get_variant,
            // Audio Information
            crate::commands::audio::get_audio_device,
            crate::commands::audio::change_audio_device,
            crate::commands::audio::stop_audio_device,
            crate::commands::audio::get_devices,
            // Stream Information
            crate::commands::network::stop_network_stream,
            crate::commands::network::change_network_stream
        ])
        .setup(|app| {
            log::info!("BVC Variant {:?}", crate::commands::env::get_variant());
            // Initialize Stronghold so we can use it to store secrets
            let secret_store = app.store("secrets.json")?;
            let stronghold_salt = match secret_store.get("stronghold_password") {
                Some(salt) => match salt.get("value") {
                    Some(salt) => Some(salt.to_string()),
                    None => None,
                },
                None => None,
            };

            if stronghold_salt.is_none() {
                let salt = common::ncryptflib::randombytes_buf(64);
                let encoded_salt = base64::encode(salt);
                secret_store.set(
                    "stronghold_password",
                    json!({ "value": encoded_salt.clone() }),
                );
            }

            let handle = app.handle().clone();

            handle.plugin(
                tauri_plugin_stronghold::Builder::new(|password| {
                    // This MUST be a 32 byte output
                    let mut hasher = Blake2s256::new();
                    hasher.update(password.as_bytes());
                    return hasher.finalize().to_vec();
                })
                .build(),
            )?;

            // On Windows, and Linux, circumvent non-installed desktop application deep link
            // url handling by force registering them with the system
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }

            // Handle updates for desktop applications
            #[cfg(desktop)]
            {
                handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            }

            let store = app.store("store.json")?;
            let resource_path = app.path().resolve("data.json", BaseDirectory::Resource)?;
            let file = File::open(&resource_path).unwrap();
            let data: serde_json::Value = serde_json::from_reader(file).unwrap();

            let android_signature_hash: String;
            if cfg!(dev) {
                android_signature_hash = data["android"]["signature_hash"]["test"]
                    .as_str()
                    .unwrap()
                    .to_string();
            } else {
                android_signature_hash = data["android"]["signature_hash"]["live"]
                    .as_str()
                    .unwrap()
                    .to_string();
            }

            store.set(
                "android_signature_hash".to_string(),
                json!({ "value": android_signature_hash }),
            );

            let app_state = Mutex::new(structs::app_state::AppState::new(store.clone()));
            app.manage(app_state);

            // This is our audio producer and consumer
            // The producer is responsible for getting audio from the raw input device, then sending it to the consumer
            // The consumer lives in the networking thread, consumes the audio, then sends it to the server
            let (audio_producer, audio_consumer) = flume::bounded::<AudioPacket>(10000);
            app.manage(Arc::new(audio_producer));
            app.manage(Arc::new(audio_consumer));

            // This is our network producer and consumer
            // The producer retrieves data from the raw QUIC stream, then sends it to the consumer
            // The consumer receives the data, then pushed it to the output device
            let (quic_producer, quic_consumer) = flume::bounded::<NetworkPacket>(10000);
            app.manage(Arc::new(quic_producer));
            app.manage(Arc::new(quic_consumer));

            let audio_stream = AudioStreamManager::new(
                handle.state::<Arc<Sender<NetworkPacket>>>().inner().clone(),
                handle.state::<Arc<Receiver<AudioPacket>>>().inner().clone(),
            );
            app.manage(Mutex::new(audio_stream));

            // This is necessary to setup s2n_quic. It doesn't need to be called elsewhere
            _ = s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
                .install_default();

            let network_stream = NetworkStreamManager::new(
                handle.state::<Arc<Sender<AudioPacket>>>().inner().clone(),
                handle.state::<Arc<Receiver<NetworkPacket>>>().inner().clone(),
            );
            app.manage(AsyncMutex::new(network_stream));

            crate::events::register(app);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
