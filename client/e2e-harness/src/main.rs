// End-to-end harness bin. Boots the REAL Tauri (Wry) client with a HIDDEN
// window and the Fake audio backend, then speaks a length-prefixed JSON frame
// protocol over stdin/stdout. Gated behind the `e2e` feature so normal builds
// never compile it.
//
// The standalone smoke (no server env) emits `Ready` and exits cleanly on
// `Shutdown`; supplying the BVC_E2E_* variables drives the full connect and
// PCM-streaming sequence against a live server.

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_store::StoreExt;

// `AppHandle::exit` terminates the tao event loop without unwinding through the
// CRT's atexit table, so the coverage runtime's writer never fires and the
// process contributes no profile data. Flushing explicitly at each exit site is
// what makes this bin measurable. `coverage` is set by cargo-llvm-cov.
#[cfg(coverage)]
unsafe extern "C" {
    fn __llvm_profile_write_file() -> i32;
}

#[cfg(coverage)]
struct CoverageFlush;

#[cfg(coverage)]
impl CoverageFlush {
    fn flush() {
        unsafe {
            __llvm_profile_write_file();
        }
    }
}

use bvc_client_lib::app_builder::AppBuilder;
use bvc_client_lib::audio::AudioBackend;
use bvc_client_lib::testkit::bridge::{Frame, InMsg, OutMsg};
use bvc_client_lib::testkit::connect::Connector;
use bvc_client_lib::{BridgeInputSource, CapturingSink};

mod channel_driver;
mod e2e_env;
mod stdout_bridge;
mod store_seeder;

use channel_driver::ChannelDriver;
use e2e_env::E2eEnv;
use stdout_bridge::StdoutBridge;
use store_seeder::StoreSeeder;

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

    // Move app_data_dir into a throwaway e2e namespace so every identifier-scoped
    // write — the seeded store, the audio input path's own `app.store("store.json")`
    // read, the webview's store/cookies — lands off the real client's app-data dir.
    // The harness wipes this namespace between runs.
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "com.alaydriem.bvc.client.e2e".to_string();
    // Headless: drop the configured window so no WebView2 instance is created. This
    // removes the dominant per-process cost (each WebView2 spawns several helper
    // processes), letting many client procs run without exhausting resources. The
    // test driver is entirely Rust over the AppHandle + managed State.
    context.config_mut().app.windows.clear();

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

            // No window is created (cleared from the context above), so there is
            // nothing to hide here — the bin runs headless.

            // The audio input path (and others) read `app.store("store.json")`
            // resolved under app_data_dir, so the seed must use that same relative
            // path — the e2e identifier override in `main` is what keeps app_data_dir
            // (and therefore this store) out of the production namespace.
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
            let harness_announce_injector = Connector::announce_injector();
            #[cfg(feature = "bedrock-protocol")]
            {
                app.manage(Connector::bedrock_state());
                app.manage(Connector::feature_flag_service());
                app.manage(Arc::clone(&harness_beacon_cache));
                app.manage(Arc::clone(&harness_eject_injector));
                app.manage(Arc::clone(&harness_presence_injector));
                app.manage(Arc::clone(&harness_announce_injector));
                app.manage(Connector::connect_error_channel());
                app.manage(Connector::chat_channel());
                app.manage(Connector::chat_injector());
            }

            AppBuilder::build_managed_state(
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
                #[cfg(feature = "bedrock-protocol")]
                Some(Arc::clone(&harness_announce_injector)),
            )?;

            // Self-state reporter: poll the client's audio-control state and emit
            // OutMsg::State on change, so the orchestrator can assert control effects
            // (e.g. a ClientBound ClientAction muting the actor).
            let state_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let actions = bvc_client_lib::audio::AudioActionsManager::new(state_handle);
                let mut last: Option<(bool, bool, bool)> = None;
                loop {
                    let s = actions.query_state().await;
                    let cur = (s.muted, s.deafened, s.recording);
                    if last != Some(cur) {
                        last = Some(cur);
                        StdoutBridge::emit(&OutMsg::State {
                            muted: s.muted,
                            deafened: s.deafened,
                            recording: s.recording,
                        });
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                }
            });

            // Forward the gain-store-updated event — the one the dashboard's
            // player cards re-render on — with the store contents at that
            // moment, so the orchestrator can assert the render trigger and the
            // state a card would show.
            let gain_handle = handle.clone();
            // Serialises the snapshot-and-emit pairs below, so the orchestrator's single
            // "latest store" slot cannot be moved backwards by a task that read earlier but
            // finished later.
            let gain_order = std::sync::Arc::new(tauri::async_runtime::Mutex::new(()));
            tauri::Listener::listen(
                &handle.clone(),
                bvc_client_lib::events::event::player_gain_store::PLAYER_GAIN_STORE_UPDATED,
                move |_| {
                    // Spawned, never blocked on. This callback runs on the async runtime's own
                    // worker thread, and `block_on` there panics with "cannot start a runtime
                    // from within a runtime". The panic is the dangerous part rather than the
                    // lost emit: it unwinds while the event listener registry mutex is held,
                    // poisoning it, after which every later emit in the process silently falls
                    // into the pending queue and no Rust-side event is ever delivered again.
                    // The orchestrator polls with a timeout, so emitting a moment later costs
                    // nothing.
                    let handle = gain_handle.clone();
                    let order = gain_order.clone();
                    tauri::async_runtime::spawn(async move {
                        // Read and emit under one lock. Two events spawn two tasks, and
                        // without this the slower one can publish an older snapshot after the
                        // newer one — the orchestrator keeps only the latest, so it would sit
                        // on stale state until the test timed out.
                        let _ordered = order.lock().await;
                        // Read from the settings service, which owns these now. The
                        // projection is keyed on identity and scoped to the current server,
                        // so this is the same shape and contents the mixer is handed.
                        let store_json = match tauri::Manager::try_state::<
                            std::sync::Arc<bvc_client_lib::players::PlayerSettingsCoordinator>,
                        >(&handle)
                        {
                            Some(coordinator) => coordinator
                                .store_for_current_server(&handle)
                                .await
                                .ok()
                                .and_then(|gains| serde_json::to_string(&gains).ok())
                                .unwrap_or_else(|| "{}".to_string()),
                            None => "{}".to_string(),
                        };
                        StdoutBridge::emit(&OutMsg::GainStoreUpdated { store_json });
                    });
                },
            );

            // Forward the remaining frontend-facing control/UI events verbatim.
            // Every name here must match a `listen()` call in the desktop
            // frontend — these are the render triggers the webview consumes,
            // surfaced so scenarios can assert them at the boundary.
            // (audio-activity is deliberately absent: it fires at speech rate
            // and would flood the bridge.)
            const FORWARDED_UI_EVENTS: &[&str] = &[
                "mute:input",
                "mute:output",
                "channel_event",
                "player_presence",
                "recording:started",
                "recording:stopped",
                "connection_health",
            ];
            for name in FORWARDED_UI_EVENTS {
                let event_name = (*name).to_string();
                tauri::Listener::listen(&handle.clone(), *name, move |event| {
                    StdoutBridge::emit(&OutMsg::UiEvent {
                        event: event_name.clone(),
                        payload: event.payload().to_string(),
                    });
                });
            }

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
                            #[cfg(coverage)]
                            CoverageFlush::flush();
                            stdin_handle.exit(0);
                            break;
                        }
                        Ok(InMsg::RequestDiagnostics) => {
                            let service = stdin_handle
                                .try_state::<std::sync::Arc<
                                    bvc_client_lib::diagnostics::LinkDiagnosticsService,
                                >>();
                            let snapshot = service.and_then(|s| s.snapshot());
                            StdoutBridge::emit(&OutMsg::Diagnostics {
                                connected: snapshot.is_some(),
                                stalled: snapshot
                                    .as_ref()
                                    .map(|s| s.link.stalled)
                                    .unwrap_or(false),
                                uptime_secs: snapshot
                                    .as_ref()
                                    .map(|s| s.link.uptime_secs)
                                    .unwrap_or(0),
                                datagrams_sent: snapshot
                                    .as_ref()
                                    .map(|s| s.mic.datagrams_per_sec as u64)
                                    .unwrap_or(0),
                                datagrams_received: snapshot
                                    .as_ref()
                                    .map(|s| s.playback.datagrams_per_sec as u64)
                                    .unwrap_or(0),
                                transport: snapshot
                                    .as_ref()
                                    .and_then(|s| s.session.transport)
                                    .map(|t| t.as_str().to_string()),
                                peers: snapshot
                                    .as_ref()
                                    .map(|s| {
                                        s.peers.iter().map(|p| p.name.clone()).collect()
                                    })
                                    .unwrap_or_default(),
                                downlink_loss_pct: snapshot
                                    .as_ref()
                                    .and_then(|s| s.link.downlink_loss_pct),
                            });
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
                        Ok(InMsg::StartProxy {
                            upstream_host,
                            upstream_port,
                            listen_port,
                            addon_transport,
                        }) => {
                            let h = stdin_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                match bvc_client_lib::testkit::connect::Connector::start_proxy(
                                    &h,
                                    upstream_host,
                                    upstream_port,
                                    listen_port,
                                    addon_transport,
                                )
                                .await
                                {
                                    Ok(()) => {
                                        StdoutBridge::emit(&OutMsg::ProxyStarted { listen_port })
                                    }
                                    Err(e) => StdoutBridge::emit(&OutMsg::Log {
                                        line: format!("start_proxy failed: {e}"),
                                    }),
                                }
                            });
                        }
                        #[cfg(not(feature = "bedrock-protocol"))]
                        Ok(InMsg::StartProxy { .. }) => {
                            StdoutBridge::emit(&OutMsg::Log {
                                line: "start_proxy: bedrock-protocol feature not enabled"
                                    .to_string(),
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            #[cfg(coverage)]
                            CoverageFlush::flush();
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
        .build(context)
        .expect("error while building e2e tauri application")
        .run(|_app_handle, _event| {});
}
