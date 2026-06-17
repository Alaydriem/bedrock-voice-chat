// End-to-end harness bin. Boots the REAL Tauri (Wry) client with a HIDDEN
// window and the Fake audio backend, then speaks a length-prefixed JSON frame
// protocol over stdin/stdout. Gated behind the `e2e` feature so normal builds
// never compile it.
//
// The standalone smoke (no server env) emits `Ready` and exits cleanly on
// `Shutdown`; supplying the BVC_E2E_* variables drives the full connect and
// PCM-streaming sequence against a live server.

use std::io::Write as _;
use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_store::StoreExt;

use bvc_client_lib::app_builder::{AudioBackend, build_managed_state};
use bvc_client_lib::testkit::bridge::{Frame, InMsg, OutMsg};
use bvc_client_lib::testkit::connect::{ConnectConfig, Connector};
use bvc_client_lib::{BridgeInputSource, CapturingSink};

// Stdout is shared between the capture-drain thread and any ad-hoc logging, so
// every framed write goes through one lock to avoid interleaving frames.
struct StdoutBridge;

impl StdoutBridge {
    fn emit(msg: &OutMsg) {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        if Frame::write(&mut lock, msg).is_err() {
            return;
        }
        let _ = lock.flush();
    }
}

// Drives an explicit channel-membership operation on the Tauri runtime and
// emits a completion frame. Spawned onto the async runtime so the synchronous
// stdin reader thread is never blocked on network I/O.
struct ChannelDriver;

impl ChannelDriver {
    fn run(
        handle: &tauri::AppHandle,
        channel_id: String,
        event: common::structs::channel::ChannelEvents,
        op: &'static str,
    ) {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            match Connector::channel_event(&handle, channel_id, event).await {
                Ok(()) => StdoutBridge::emit(&OutMsg::ChannelOpDone { op: op.to_string() }),
                Err(e) => StdoutBridge::emit(&OutMsg::Log {
                    line: format!("channel {op} failed: {e}"),
                }),
            }
        });
    }
}

// Reads the four optional BVC_E2E_* variables. The connect sequence only runs
// when a login code is present; the standalone smoke leaves it unset.
struct E2eEnv;

impl E2eEnv {
    fn connect_config() -> Option<ConnectConfig> {
        let code = std::env::var("BVC_E2E_CODE").ok().filter(|s| !s.is_empty())?;
        Some(ConnectConfig {
            server: std::env::var("BVC_E2E_SERVER").unwrap_or_default(),
            gamertag: std::env::var("BVC_E2E_GAMERTAG").unwrap_or_default(),
            code,
            channel: std::env::var("BVC_E2E_CHANNEL").ok().filter(|s| !s.is_empty()),
            channel_id: std::env::var("BVC_E2E_CHANNEL_ID").ok().filter(|s| !s.is_empty()),
        })
    }
}

// Seeds the store so AppState construction and the audio device setup never
// touch real Cpal devices. The Fake backend bypasses device enumeration, but
// AppState still reads these keys during construction.
struct StoreSeeder;

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

    fn seed(store: &Arc<tauri_plugin_store::Store<tauri::Wry>>) {
        store.set("current_player", serde_json::json!("E2ePlayer"));
        store.set("input_audio_device", Self::fake_device("input_audio_device"));
        store.set("output_audio_device", Self::fake_device("output_audio_device"));
        store.set("install_id", serde_json::json!("00000000-0000-0000-0000-000000000000"));
        store.set("use_noise_gate", serde_json::json!(false));
        let _ = store.save();
    }
}

fn main() {
    _ = common::s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default();

    let _ = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[e2e][{}] {}", record.level(), message))
        })
        .level(log::LevelFilter::Warn)
        .chain(std::io::stderr())
        .apply();


    // input_tx feeds the real DSP through BridgeInputSource; cap_rx receives
    // post-mix PCM from CapturingSink.
    let (input_tx, input_rx) = flume::unbounded::<Vec<f32>>();
    let (cap_tx, cap_rx) = flume::unbounded::<Vec<f32>>();

    let input = BridgeInputSource::new(input_rx, 48_000, 1);
    let capture = CapturingSink::new(cap_tx, 48_000, 2);

    let mut backend = Some(AudioBackend::Fake { input, capture });
    let mut cap_rx = Some(cap_rx);
    let mut input_tx = Some(input_tx);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(
            tauri_plugin_store::Builder::default()
                .default_serialize_fn(|cache| {
                    let sorted: std::collections::BTreeMap<_, _> = cache.iter().collect();
                    serde_json::to_vec_pretty(&sorted).map_err(Into::into)
                })
                .build(),
        )
        .setup(move |app| {
            let handle = app.handle().clone();

            // tauri.conf.json declares a `main` window that Tauri auto-creates,
            // so hide that one rather than building a second `main`. Fall back to
            // building a hidden window if no config window is present.
            match app.get_webview_window("main") {
                Some(window) => {
                    let _ = window.hide();
                }
                None => {
                    tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                        .visible(false)
                        .build()?;
                }
            }

            let store = app.store("store.json")?;
            StoreSeeder::seed(&store);

            let app_state = bvc_client_lib::AppState::new(store.clone(), handle.clone());
            app.manage(tauri::async_runtime::Mutex::new(app_state));

            // The connect commands read an AnalyticsService from State; register a
            // telemetry-disabled instance so the real command path runs unchanged.
            app.manage(Connector::analytics_service());

            // Bedrock proxy commands read several State entries that the production
            // `run()` registers but the harness bin must register explicitly. Each
            // factory builds a no-op or default instance: FeatureFlagService uses an
            // empty API key (no remote refresh), BedrockState starts empty (the stub
            // auth manager is seeded by Connector::start_proxy on demand).
            //
            // beacon_cache, eject_injector, and presence_injector are constructed
            // here so the same Arc is shared with build_managed_state below — the
            // output stream manager calls beacon_cache.observe() as jukebox frames
            // arrive, which is what lets the PlaySoundHandler resolve an event_id
            // for bvc:eject. Without this wiring the eject silently drops.
            #[cfg(feature = "bedrock-protocol")]
            let harness_beacon_cache = Connector::beacon_cache();
            #[cfg(feature = "bedrock-protocol")]
            let harness_eject_injector = Connector::eject_injector();
            #[cfg(feature = "bedrock-protocol")]
            let harness_presence_injector = Connector::presence_injector();
            #[cfg(feature = "bedrock-protocol")]
            {
                app.manage(Connector::bedrock_state());
                app.manage(Connector::feature_flag_service());
                app.manage(Arc::clone(&harness_beacon_cache));
                app.manage(Arc::clone(&harness_eject_injector));
                app.manage(Arc::clone(&harness_presence_injector));
                app.manage(Connector::connect_error_channel());
            }

            build_managed_state(
                app,
                backend.take().expect("backend taken once"),
                #[cfg(feature = "bedrock-protocol")]
                None,
                #[cfg(feature = "bedrock-protocol")]
                Some(Arc::clone(&harness_beacon_cache)),
                #[cfg(feature = "bedrock-protocol")]
                Some(Arc::clone(&harness_eject_injector)),
                #[cfg(feature = "bedrock-protocol")]
                Some(Arc::clone(&harness_presence_injector)),
            )?;

            // Capture-drain thread: post-mix PCM out to stdout as it arrives.
            let cap_rx = cap_rx.take().expect("cap_rx taken once");
            std::thread::spawn(move || {
                while let Ok(samples) = cap_rx.recv() {
                    StdoutBridge::emit(&OutMsg::CapturedPcm { samples });
                }
            });

            // Stdin reader thread: decode InMsg frames and route them.
            let input_tx = input_tx.take().expect("input_tx taken once");
            let stdin_handle = handle.clone();
            std::thread::spawn(move || {
                let mut stdin = std::io::stdin().lock();
                loop {
                    match Frame::read::<_, InMsg>(&mut stdin) {
                        Ok(InMsg::InputPcm { samples }) => {
                            let _ = input_tx.send(samples);
                        }
                        Ok(InMsg::Disconnect) => {
                            let disconnect_handle = stdin_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                match Connector::disconnect(&disconnect_handle).await {
                                    Ok(()) => StdoutBridge::emit(&OutMsg::Disconnected),
                                    Err(e) => StdoutBridge::emit(&OutMsg::Log {
                                        line: format!("disconnect failed: {e}"),
                                    }),
                                }
                            });
                        }
                        Ok(InMsg::LeaveChannel { channel_id }) => {
                            ChannelDriver::run(
                                &stdin_handle,
                                channel_id,
                                common::structs::channel::ChannelEvents::Leave,
                                "leave",
                            );
                        }
                        Ok(InMsg::RejoinChannel { channel_id }) => {
                            ChannelDriver::run(
                                &stdin_handle,
                                channel_id,
                                common::structs::channel::ChannelEvents::Join,
                                "rejoin",
                            );
                        }
                        Ok(InMsg::DeleteChannel { channel_id }) => {
                            ChannelDriver::run(
                                &stdin_handle,
                                channel_id,
                                common::structs::channel::ChannelEvents::Delete,
                                "delete",
                            );
                        }
                        Ok(InMsg::UploadAudio { wav_path, game }) => {
                            let h = stdin_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                match Connector::upload_audio(&h, wav_path, game).await {
                                    Ok(resp) => StdoutBridge::emit(&OutMsg::AudioUploaded {
                                        audio_file_id: resp.id,
                                        duration_ms: resp.duration_ms.max(0) as u32,
                                    }),
                                    Err(e) => StdoutBridge::emit(&OutMsg::Log {
                                        line: format!("upload_audio failed: {e}"),
                                    }),
                                }
                            });
                        }
                        Ok(InMsg::Shutdown) => {
                            stdin_handle.exit(0);
                            break;
                        }
                        Ok(InMsg::RequestStats) => {
                            let (sent, from_quic, into_jitter_buffer) =
                                bvc_client_lib::testkit::counters::TransportCounters::snapshot();
                            StdoutBridge::emit(&OutMsg::Stats {
                                frames_sent: sent,
                                frames_from_quic: from_quic,
                                frames_into_jitter_buffer: into_jitter_buffer,
                            });
                        }
                        Ok(InMsg::TriggerJukebox { .. }) | Ok(InMsg::InjectPresence { .. }) => {
                            StdoutBridge::emit(&OutMsg::Log {
                                line: "received unhandled command".to_string(),
                            });
                        }
                        #[cfg(feature = "bedrock-protocol")]
                        Ok(InMsg::StartProxy { upstream_host, upstream_port, listen_port }) => {
                            let h = stdin_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                match bvc_client_lib::testkit::connect::Connector::start_proxy(
                                    &h,
                                    upstream_host,
                                    upstream_port,
                                    listen_port,
                                )
                                .await
                                {
                                    Ok(()) => StdoutBridge::emit(&OutMsg::ProxyStarted { listen_port }),
                                    Err(e) => StdoutBridge::emit(&OutMsg::Log {
                                        line: format!("start_proxy failed: {e}"),
                                    }),
                                }
                            });
                        }
                        #[cfg(not(feature = "bedrock-protocol"))]
                        Ok(InMsg::StartProxy { .. }) => {
                            StdoutBridge::emit(&OutMsg::Log {
                                line: "start_proxy: bedrock-protocol feature not enabled".to_string(),
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            stdin_handle.exit(0);
                            break;
                        }
                        Err(e) => {
                            StdoutBridge::emit(&OutMsg::Log {
                                line: format!("stdin decode error: {e}"),
                            });
                        }
                    }
                }
            });

            // Connect sequence only when a login code is supplied.
            if let Some(config) = E2eEnv::connect_config() {
                let connect_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    match Connector::run(&connect_handle, &config).await {
                        Ok(channel_id) => {
                            if let Some(channel_id) = channel_id {
                                StdoutBridge::emit(&OutMsg::ChannelJoined { channel_id });
                            }
                            StdoutBridge::emit(&OutMsg::Connected)
                        }
                        Err(e) => StdoutBridge::emit(&OutMsg::Log {
                            line: format!("connect failed: {e}"),
                        }),
                    }
                });
            }

            StdoutBridge::emit(&OutMsg::Ready);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building e2e tauri application")
        .run(|_app_handle, _event| {});
}
