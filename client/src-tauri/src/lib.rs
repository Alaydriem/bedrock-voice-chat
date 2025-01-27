use common::ncryptflib::rocket::base64;
use tauri::Manager;
use serde_json::json;
use std::fs::File;
use tauri::path::BaseDirectory;
use tauri_plugin_store::StoreExt;
use blake2::{Blake2s256, Digest};
use log::{info, error};

mod auth;
mod audio;
mod network;
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // For desktop applications, enforce only a single running instance at a time
    // And enable logging
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main")
            .expect("no main window")
            .set_focus();
        }));

        builder = builder.plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
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
            crate::auth::commands::server_login,
            crate::commands::env::get_env,
            crate::commands::env::get_variant
        ])
        .setup(|app| {

            log::info!("{:?}",  crate::commands::env::get_variant());
            // Initialize Stronghold so we can use it to store secrets
            let secret_store = app.store("secrets.json")?;
            match secret_store.get("stronghold_password") {
                Some(salt) => salt.get("value").unwrap().to_string(),
                None => {
                    let salt = common::ncryptflib::randombytes_buf(64);
                    let encoded_salt = base64::encode(salt);
                    secret_store.set("stronghold_password", json!({ "value": encoded_salt.clone() }));
                    encoded_salt
                }
            };

            app.handle().plugin(tauri_plugin_stronghold::Builder::new(|password| {
                // This MUST be a 32 byte output
                let mut hasher = Blake2s256::new();
                hasher.update(password.as_bytes());
                return hasher.finalize().to_vec();
            }).build())?;

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
                app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            }

            let store = app.store("store.json")?;
            let resource_path = app.path().resolve("data.json", BaseDirectory::Resource)?;
            let file = File::open(&resource_path).unwrap();
            let data: serde_json::Value = serde_json::from_reader(file).unwrap();

            let android_signature_hash: String;
            if cfg!(dev)
            {
                android_signature_hash = data["android"]["signature_hash"]["test"].as_str().unwrap().to_string();
            } else {
                android_signature_hash = data["android"]["signature_hash"]["live"].as_str().unwrap().to_string(); 
            }

            store.set("android_signature_hash".to_string(), json!({ "value": android_signature_hash }));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
