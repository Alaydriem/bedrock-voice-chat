use common::consts::variant::{Variant, get_variant};
use crate::structs::app_state::AppState;
use audio::AudioPacket;
use flume::{Receiver, Sender};
use network::NetworkPacket;
use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use tauri_plugin_deep_link::DeepLinkExt;
use log::{info, error, warn};
use audio::AudioStreamManager;
use audio::recording::RecordingManager;
use network::NetworkStreamManager;

use common::structs::DeepLink;
use deep_links::DeepLinkHandler;

mod api;
pub mod audio;
mod auth;
mod commands;
mod core;
mod deep_links;
mod events;
#[cfg(desktop)]
pub mod keybinds;
mod logging;
mod network;
mod structs;
pub mod websocket;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    _ = common::s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default();

    let sentry_guard = sentry::init((
        option_env!("SENTRY_DSN").unwrap_or(""),
        sentry::ClientOptions {
            release: Some(env!("SENTRY_RELEASE").into()),
            enable_logs: true,
            environment: Some(
                match get_variant() {
                    Variant::Dev => "development",
                    Variant::Release => "production"
                }.into()
            ),
            ..Default::default()
        },
    ));

    #[cfg(desktop)]
    let _minidump = sentry_rust_minidump::init(&sentry_guard)
        .map_err(|e| warn!("Minidump crash reporter failed to initialize: {e}"))
        .ok();

    let mut builder = tauri::Builder::default();

    // For desktop applications, enforce only a single running instance at a time
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }));
    }

    builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(
                    std::env::var("LOG_LEVEL")
                        .ok()
                        .and_then(|s| s.parse::<log::LevelFilter>().ok())
                        .unwrap_or(log::LevelFilter::Info),
                )
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stderr),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: None }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Dispatch(
                        fern::Dispatch::new()
                            .chain(Box::new(logging::SentryLogger::new()) as Box<dyn log::Log>),
                    )),
                ])
                .build()
        )
        .plugin(tauri_plugin_audio_permissions::init())
        .plugin(tauri_plugin_keyring::init())
        //.plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            // About
            crate::commands::about::get_app_info,
            crate::commands::about::export_logs,
            // Authentication
            crate::auth::commands::server_login,
            crate::auth::commands::logout,
            crate::auth::commands::start_hytale_device_flow,
            crate::auth::commands::poll_hytale_status,
            crate::auth::commands::code_login,
            // Environment Variable Data
            crate::commands::env::get_env,
            crate::commands::env::get_variant,
            // Audio Information
            crate::commands::audio::get_audio_device,
            crate::commands::audio::set_audio_device,
            crate::commands::audio::change_audio_device,
            crate::commands::audio::stop_audio_device,
            crate::commands::audio::get_devices,
            crate::commands::audio::mute,
            crate::commands::audio::mute_status,
            crate::commands::audio::is_stopped,
            crate::commands::audio::update_stream_metadata,
            crate::commands::audio::reset_asm,
            crate::commands::audio::start_recording,
            crate::commands::audio::stop_recording,
            crate::commands::audio::get_recording_status,
            crate::commands::audio::is_recording,
            crate::commands::audio::get_current_players,
            crate::commands::audio::restart_audio_stream,
            // Recordings Management
            crate::commands::recordings::get_recording_sessions,
            crate::commands::recordings::delete_recording_session,
            crate::commands::recordings::export_recording,
            // Stream Information
            crate::commands::network::stop_network_stream,
            crate::commands::network::change_network_stream,
            crate::commands::network::reset_nsm,
            // API implementation
            crate::api::commands::api_initialize_client,
            crate::api::commands::api_ping,
            crate::api::commands::api_get_config,
            crate::api::commands::api_create_channel,
            crate::api::commands::api_delete_channel,
            crate::api::commands::api_list_channels,
            crate::api::commands::api_get_channel,
            crate::api::commands::api_channel_event,
            crate::api::commands::api_rename_channel,
            crate::api::commands::api_get_player_gamerpic,
            // WebSocket Server
            crate::commands::websocket::update_websocket_config,
            crate::commands::websocket::start_websocket_server,
            crate::commands::websocket::stop_websocket_server,
            crate::commands::websocket::is_websocket_running,
            crate::commands::websocket::generate_encryption_key,
            // Keybinds
            crate::commands::keybinds::start_keybind_listener,
            // Updater
            #[cfg(desktop)]
            crate::commands::updater::check_for_updates
        ])
        .setup(|app| {
            // Set Windows timer resolution for high-precision audio timing
            #[cfg(target_os = "windows")]
            {
                windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
                windows_targets::link!("ntdll.dll" "system" fn NtQueryTimerResolution(
                    minimumresolution: *mut u32,
                    maximumresolution: *mut u32,
                    currentresolution: *mut u32,
                ) -> i32);

                let mut min_res = 0u32;
                let mut max_res = 0u32;
                let mut current_res = 0u32;
                unsafe {
                    NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
                    let current_ms = current_res as f64 / 10_000.0;
                    info!("Current Windows timer resolution: {:.2}ms", current_ms);

                    timeBeginPeriod(1);

                    NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
                    let new_ms = current_res as f64 / 10_000.0;
                    info!("Set Windows timer resolution to 1ms (actual: {:.2}ms)", new_ms);

                    if new_ms > 2.0 {
                        warn!("WARNING: Timer resolution is degraded ({:.2}ms). This will cause audio jitter!", new_ms);
                        warn!("Try closing other applications or restarting Windows.");
                    }
                }
            }

            info!("BVC Variant {:?}", crate::commands::env::get_variant());
            info!("Protocol Version: {}", common::consts::version::PROTOCOL_VERSION);
            let sentry_enabled = sentry::Hub::current().client().map(|c| c.is_enabled()).unwrap_or(false);
            info!("Sentry: {}", if sentry_enabled { "initialized" } else { "not configured (DSN missing or invalid)" });
            let store = app.store("store.json")?;

            if sentry_enabled {
                let install_id = match store.get("install_id").and_then(|v| v.as_str().map(String::from)) {
                    Some(id) => id,
                    None => {
                        let id = uuid::Uuid::now_v7().to_string();
                        store.set("install_id", id.clone());
                        let _ = store.save();
                        id
                    }
                };
                sentry::configure_scope(|scope| {
                    scope.set_user(Some(sentry::User {
                        id: Some(install_id),
                        ..Default::default()
                    }));
                });
            }

            let handle = app.handle().clone();

            // Register deep links for Desktop targets
            #[cfg(any(windows, target_os = "linux"))]
            {
                app.deep_link().register_all()?;
            }

            // Register event handler for incoming deep links (when app is already running)
            let app_handle = handle.clone();
            app.deep_link().on_open_url(move |event| {
                info!("on_open_url callback fired");
                for url in event.urls() {
                    info!("Processing deep link URL from on_open_url: {}", url);
                    let deep_link = DeepLink::new(url.to_string());
                    if let Err(e) = deep_link.handle(&app_handle) {
                        error!("Failed to handle deep link: {}", e);
                    } else {
                        info!("Successfully handled deep link from on_open_url");
                    }
                }
            });

            // Check for deep links that cold-started the app
            let app_handle2 = handle.clone();
            match app.deep_link().get_current() {
                Ok(urls) => match urls {
                    Some(urls) => {
                        for url in &urls {
                            let deep_link = DeepLink::new(url.to_string());
                            if let Err(e) = deep_link.handle(&app_handle2) {
                                error!("Failed to handle cold-start deep link: {}", e);
                            } else {
                                info!("Successfully handled cold-start deep link");
                            }
                        }
                    },
                    None => {
                        info!("No cold-start deep links found");
                    }
                }
                Err(e) => {
                    warn!("cold-start deep links error: {}", e);
                }
            }

            // Register the updater plugin. Actual update checks happen via the
            // check_for_updates command which uses UpdaterExt with dynamic endpoints.
            #[cfg(desktop)]
            {
                handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            }

            let app_state = AppState::new(store.clone(), handle.clone());
            app.manage(Mutex::new(app_state));

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

            // This is our RecordingManager
            // It is responsible for managing recording sessions and owns internal producer/consumer channels
            // for both the input and output stream
            let recording_manager = RecordingManager::new(handle.clone());
            app.manage(Arc::new(Mutex::new(recording_manager)));

            // Create AudioStreamManager with RecordingManager reference
            let audio_stream = AudioStreamManager::new(
                handle.state::<Arc<Sender<NetworkPacket>>>().inner().clone(),
                handle.state::<Arc<Receiver<AudioPacket>>>().inner().clone(),
                handle.clone(),
                Some(handle.state::<Arc<Mutex<RecordingManager>>>().inner().clone()),
            );
            app.manage(Mutex::new(audio_stream));

            // Initialize WebSocketManager and register the broadcaster
            let ws_manager = websocket::WebSocketManager::new(handle.clone());
            let ws_broadcaster = ws_manager.broadcaster();
            app.manage(ws_broadcaster);
            app.manage(Mutex::new(ws_manager));

            // AudioActionsManager handles mute, deafen, and recording state changes for both user-initiated actions (keybinds) and API calls
            let audio_actions = crate::audio::AudioActionsManager::new(handle.clone());
            app.manage(audio_actions);

            // KeybindManager listens for global key events and triggers actions in AudioActionsManager for desktop
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::ShortcutState;

                let action_map: Arc<parking_lot::RwLock<keybinds::ActionMap>> =
                    Arc::new(parking_lot::RwLock::new(Vec::new()));
                let listener = Arc::new(keybinds::listener::KeybindListener::new(handle.clone()));

                let handler_map = action_map.clone();
                let handler_listener = listener.clone();

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, shortcut, event| {
                            let action = handler_map
                                .read()
                                .iter()
                                .find(|(s, _)| s == shortcut)
                                .map(|(_, a)| a.clone());
                            if let Some(action) = action {
                                let l = handler_listener.clone();
                                let state = event.state();
                                tauri::async_runtime::spawn(async move {
                                    match state {
                                        ShortcutState::Pressed => l.on_action_press(action).await,
                                        ShortcutState::Released => {
                                            l.on_action_release(action).await
                                        }
                                    }
                                });
                            }
                        })
                        .build(),
                )?;

                let keybind_manager =
                    keybinds::KeybindManager::new(handle.clone(), listener, action_map);
                app.manage(keybind_manager);
            }

            let network_stream = NetworkStreamManager::new(
                handle.state::<Arc<Sender<AudioPacket>>>().inner().clone(),
                handle
                    .state::<Arc<Receiver<NetworkPacket>>>()
                    .inner()
                    .clone(),
                handle.clone(),
            );
            app.manage(Mutex::new(network_stream));

            // Event Handlers
            crate::events::Notification::register(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}