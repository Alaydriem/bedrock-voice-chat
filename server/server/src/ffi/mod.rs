//! FFI exports for embedding BVC server in other applications (e.g., JNI for Kotlin/Java).
//!
//! # Threading Model
//! - Java/Kotlin owns the thread that calls `bvc_server_start()`
//! - `bvc_server_start()` blocks until shutdown
//! - `bvc_server_stop()` can be called from any thread to signal shutdown
//!
//! # Usage from JNI
//! ```java
//! long handle = BvcNative.createServer(configJson);
//! // Start in dedicated thread - this blocks
//! new Thread(() -> BvcNative.startServer(handle)).start();
//! // Later, signal shutdown from any thread
//! BvcNative.stopServer(handle);
//! // After start() returns, destroy the handle
//! BvcNative.destroyServer(handle);
//! ```

mod error;

use error::FfiError;

use crate::config::ApplicationConfig;
use crate::runtime::{ServerRuntime, position_updater};
use crate::services::{
    AudioPlaybackService, AuthCodeService, PlayerIdentityService, PlayerRegistrarService,
};
use crate::stream::quic::WebhookReceiver;

use common::Game;
use common::traits::player_data::PlayerData;
use sea_orm::DatabaseConnection;
use std::ffi::{CStr, CString, c_char, c_int};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// Opaque handle to a server runtime instance
pub struct RuntimeHandle {
    runtime: Mutex<Option<ServerRuntime>>,
    tokio_runtime: Option<tokio::runtime::Runtime>,
    /// Shutdown flag - accessible without locking runtime mutex
    shutdown_flag: Arc<AtomicBool>,
    shutdown_notify: Arc<tokio::sync::Notify>,
    /// Webhook receiver for position updates - accessible without locking runtime mutex
    webhook_receiver: Arc<RwLock<Option<WebhookReceiver>>>,
    /// Cache manager for FFI control-plane routing - accessible without locking runtime mutex
    cache_manager: Arc<RwLock<Option<crate::stream::quic::CacheManager>>>,
    /// Player registrar for player registration - accessible without locking runtime mutex
    player_registrar: Arc<RwLock<Option<PlayerRegistrarService>>>,
    /// Player identity service for cross-platform name resolution - accessible without locking runtime mutex
    identity_service: Arc<RwLock<Option<PlayerIdentityService>>>,
    /// Audio playback service - accessible without locking runtime mutex
    audio_playback_service: Arc<RwLock<Option<Arc<AudioPlaybackService>>>>,
    /// Database connection for FFI audio operations
    db_conn: Arc<RwLock<Option<Arc<DatabaseConnection>>>>,
    /// Chat hub, for an embedded mod driving chat without a socket
    chat_service: Arc<RwLock<Option<Arc<crate::services::ChatService>>>>,
    /// Metrics, for an embedded mod reporting facts about its own host
    metrics: Arc<RwLock<Option<Arc<crate::services::MetricsService>>>>,
    /// Outbound `say` frames awaiting `bvc_chat_drain`.
    ///
    /// The embedded mod is the transport here, so the queue the WebSocket route would own
    /// lives on the handle instead and is emptied by polling rather than by a socket write.
    chat_outbound: Mutex<Option<tokio::sync::mpsc::Receiver<String>>>,
    /// Every world id the embedded mod registered, so shutdown can release them all.
    chat_rooms: Mutex<Vec<String>>,
    /// Identity of the registration `chat_rooms` describes.
    ///
    /// Released against this, so a re-register followed by the older teardown cannot remove
    /// the newer registration.
    chat_socket_id: AtomicU64,
    /// The configuration the server resolved, kept so an embedder can read the
    /// values defaults and environment overrides decided.
    resolved_config: String,
    /// Whether the operator permits recording, taken from the config this handle
    /// started with. Held as a flag rather than re-parsed out of `resolved_config`,
    /// which is a JSON document read only by the embedder.
    recording_enabled: AtomicBool,
}


/// Run an FFI entry point's body, converting any panic into the function's error
/// sentinel instead of letting it unwind across the C ABI — an unwind out of an
/// `extern "C"` function aborts the host process (the Java/BDS mod's JVM). On
/// panic the message is recorded for `bvc_get_last_error` and the sentinel is
/// returned so the embedder sees a recoverable error rather than a crash.
macro_rules! ffi_guard {
    ($name:literal, $sentinel:expr, $body:block) => {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(value) => value,
            Err(_) => {
                FfiError::set_last_error(concat!("panic in ", $name));
                $sentinel
            }
        }
    };
}

/// Platform initialization. Called automatically by `bvc_server_create`.
/// Kept for backward compatibility — safe to call multiple times.
#[unsafe(no_mangle)]
pub extern "C" fn bvc_init() -> c_int {
    0
}

/// Create a server instance from JSON configuration.
///
/// # Arguments
/// * `config_json` - JSON string matching ApplicationConfig structure
///
/// # Returns
/// * Pointer to RuntimeHandle on success
/// * NULL on error (call `bvc_get_last_error()` for details)
///
/// # Safety
/// * `config_json` must be a valid null-terminated UTF-8 string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_server_create(config_json: *const c_char) -> *mut RuntimeHandle {
    ffi_guard!("bvc_server_create", ptr::null_mut(), {
    if config_json.is_null() {
        FfiError::set_last_error("config_json is null");
        return ptr::null_mut();
    }

    let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in config_json: {}", e));
            return ptr::null_mut();
        }
    };

    let config: ApplicationConfig =
        match ApplicationConfig::from_json_with_env(config_str, std::env::vars().collect()) {
            Ok(c) => c,
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to parse config JSON: {}", e));
                return ptr::null_mut();
            }
        };

    let resolved_config = match serde_json::to_string(&config) {
        Ok(json) => json,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to serialize resolved config: {}", e));
            return ptr::null_mut();
        }
    };

    let recording_enabled = config.voice.recording.enabled;

    let runtime = match crate::BvcServer::new(config) {
        Ok(r) => r,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to create runtime: {}", e));
            return ptr::null_mut();
        }
    };

    // Extract Arc clones BEFORE putting runtime in Mutex
    // This allows stop() and update_positions() to work without locking the runtime
    let shutdown_flag = runtime.shutdown_flag();
    let shutdown_notify = runtime.shutdown_notify();
    let webhook_receiver = runtime.get_webhook_receiver();
    let cache_manager = runtime.get_cache_manager();
    let player_registrar = runtime.get_player_registrar();
    let identity_service = runtime.get_identity_service();
    let audio_playback_service = runtime.get_audio_playback_service();
    let db_conn = runtime.get_db_conn();
    let chat_service = runtime.get_chat_service();
    let metrics = runtime.get_metrics();

    let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();
    runtime_builder.enable_all();
    if let Some(n) = std::env::var("BVC_RUNTIME_WORKER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
    {
        runtime_builder.worker_threads(n);
    }

    if let Some(n) = std::env::var("BVC_RUNTIME_MAX_BLOCKING_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
    {
        runtime_builder.max_blocking_threads(n);
    }

    let tokio_runtime = match runtime_builder.build() {
        Ok(rt) => rt,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to create tokio runtime: {}", e));
            return ptr::null_mut();
        }
    };

    let handle = Box::new(RuntimeHandle {
        runtime: Mutex::new(Some(runtime)),
        tokio_runtime: Some(tokio_runtime),
        shutdown_flag,
        shutdown_notify,
        webhook_receiver,
        cache_manager,
        player_registrar,
        identity_service,
        audio_playback_service,
        db_conn,
        chat_service,
        metrics,
        chat_outbound: Mutex::new(None),
        chat_rooms: Mutex::new(Vec::new()),
        chat_socket_id: AtomicU64::new(0),
        resolved_config,
        recording_enabled: AtomicBool::new(recording_enabled),
    });

    Box::into_raw(handle)
    })
}

/// Start the server. This function BLOCKS until the server stops.
///
/// Call this from a dedicated thread. Use `bvc_server_stop()` from another
/// thread to signal shutdown.
///
/// # Arguments
/// * `handle` - Handle from `bvc_server_create()`
///
/// # Returns
/// * 0 on clean shutdown
/// * -1 on error (call `bvc_get_last_error()` for details)
///
/// # Safety
/// * `handle` must be a valid pointer from `bvc_server_create()`
/// * Must not be called concurrently on the same handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_server_start(handle: *mut RuntimeHandle) -> c_int {
    ffi_guard!("bvc_server_start", -1, {
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    // Get the tokio runtime
    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            FfiError::set_last_error("Tokio runtime not available");
            return -1;
        }
    };

    // Get mutable access to the server runtime
    let mut runtime_guard = match handle_ref.runtime.lock() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to lock runtime: {}", e));
            return -1;
        }
    };

    let runtime = match runtime_guard.as_mut() {
        Some(r) => r,
        None => {
            FfiError::set_last_error("Runtime already consumed or not initialized");
            return -1;
        }
    };

    // Run the server on the tokio runtime (blocks until shutdown)
    let result = tokio_rt.block_on(async { runtime.start_async().await });

    match result {
        Ok(_) => 0,
        Err(e) => {
            FfiError::set_last_error(&format!("Server error: {}", e));
            -1
        }
    }
    })
}

/// Signal the server to stop gracefully.
///
/// This is non-blocking and can be called from any thread.
/// The `bvc_server_start()` call will return after shutdown completes.
///
/// # Arguments
/// * `handle` - Handle from `bvc_server_create()`
///
/// # Returns
/// * 0 on success
/// * -1 on error
///
/// # Safety
/// * `handle` must be a valid pointer from `bvc_server_create()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_server_stop(handle: *mut RuntimeHandle) -> c_int {
    ffi_guard!("bvc_server_stop", -1, {
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    // Use the shutdown flag directly - no mutex lock required
    // This avoids deadlock since start() holds the runtime mutex
    handle_ref.shutdown_flag.store(true, Ordering::SeqCst);
    handle_ref.shutdown_notify.notify_one();
    0
    })
}

/// Destroy the server handle and free all resources.
///
/// Call this after `bvc_server_start()` returns.
///
/// The tokio runtime shutdown is bounded by a timeout, so runtime threads may
/// briefly outlive this call. Embedders must therefore keep the library loaded
/// for the remainder of the process — unloading it (e.g. `FreeLibrary`/`dlclose`)
/// after destroy can unmap code a straggler thread is still executing.
///
/// # Arguments
/// * `handle` - Handle from `bvc_server_create()`
///
/// # Returns
/// * 0 on success
/// * -1 on error
///
/// # Safety
/// * `handle` must be a valid pointer from `bvc_server_create()`
/// * Must not be called while `bvc_server_start()` is running
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_server_destroy(handle: *mut RuntimeHandle) -> c_int {
    ffi_guard!("bvc_server_destroy", -1, {
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return -1;
    }

    // Take ownership
    let mut handle_box = unsafe { Box::from_raw(handle) };

    // Explicitly shutdown tokio runtime with timeout to avoid hanging
    // This is important because dropping a runtime waits for all tasks to complete,
    // which could block forever if tasks don't respond to cancellation
    if let Some(rt) = handle_box.tokio_runtime.take() {
        rt.shutdown_timeout(std::time::Duration::from_secs(2));
    }

    // Now drop the rest (runtime mutex, etc.)
    drop(handle_box);
    0
    })
}

/// Get the last error message.
///
/// # Returns
/// * Pointer to error string, or NULL if no error
/// * The returned string is valid until the next FFI call on the same thread
///
/// # Safety
/// * The returned pointer must not be freed by the caller
/// * The pointer is only valid until the next FFI call
#[unsafe(no_mangle)]
pub extern "C" fn bvc_get_last_error() -> *const c_char {
    FfiError::last_error_ptr()
}

/// Free a string allocated by this library.
///
/// # Safety
/// * `ptr` must be a pointer returned by this library, or NULL
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = unsafe { CString::from_raw(ptr) };
    }
}

/// Get the library version string.
///
/// # Returns
/// * Pointer to version string (static, do not free)
#[unsafe(no_mangle)]
pub extern "C" fn bvc_version() -> *const c_char {
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}

/// Get the protocol version string.
///
/// # Returns
/// * Pointer to protocol version string (static, do not free)
#[unsafe(no_mangle)]
pub extern "C" fn bvc_protocol_version() -> *const c_char {
    common::consts::version::PROTOCOL_VERSION_CSTR.as_ptr() as *const c_char
}

/// Update player positions directly via FFI
///
/// This is the preferred method for embedded mode - it avoids the HTTP
/// overhead and sends position data directly to connected QUIC clients.
///
/// # Arguments
/// * `handle` - Handle from `bvc_server_create()`
/// * `game_data_json` - JSON string matching GameDataCollection structure:
///   ```json
///   {
///     "game": "minecraft",
///     "players": [
///       {"name": "Player1", "x": 100.0, "y": 64.0, "z": 200.0, ...},
///       ...
///     ]
///   }
///   ```
///
/// # Returns
/// * 0 on success
/// * -1 on error (call `bvc_get_last_error()` for details)
///
/// # Safety
/// * `handle` must be a valid pointer from `bvc_server_create()`
/// * `game_data_json` must be a valid null-terminated UTF-8 string
/// * Server must be running (after `bvc_server_start()` has been called)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_update_positions(
    handle: *mut RuntimeHandle,
    game_data_json: *const c_char,
) -> c_int {
    ffi_guard!("bvc_update_positions", -1, {
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return -1;
    }

    if game_data_json.is_null() {
        FfiError::set_last_error("game_data_json is null");
        return -1;
    }

    let json_str = match unsafe { CStr::from_ptr(game_data_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in game_data_json: {}", e));
            return -1;
        }
    };

    // Parse the GameDataCollection JSON
    let game_data: common::GameDataCollection = match serde_json::from_str(json_str) {
        Ok(data) => data,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to parse game_data JSON: {}", e));
            return -1;
        }
    };

    let handle_ref = unsafe { &*handle };

    // Get the tokio runtime
    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            FfiError::set_last_error("Tokio runtime not available");
            return -1;
        }
    };

    // Use the webhook_receiver directly - no mutex lock required
    // This avoids deadlock since start() holds the runtime mutex
    let wr_guard = match handle_ref.webhook_receiver.read() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read webhook_receiver: {}", e));
            return -1;
        }
    };

    let webhook_receiver = match wr_guard.as_ref() {
        Some(wr) => wr,
        None => {
            FfiError::set_last_error("Server not started - webhook_receiver not available");
            return -1;
        }
    };

    // Get player registrar for registration (if available)
    let pr_guard = match handle_ref.player_registrar.read() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read player_registrar: {}", e));
            return -1;
        }
    };

    let player_registrar = pr_guard.as_ref().cloned();
    drop(pr_guard);

    // Send position update (run async operation on tokio runtime)
    // Clone the webhook_receiver reference to satisfy borrow checker
    let webhook_receiver_clone = webhook_receiver.clone();
    drop(wr_guard);

    // Get identity service for name resolution (if available)
    let is_guard = match handle_ref.identity_service.read() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read identity_service: {}", e));
            return -1;
        }
    };

    let identity_service = is_guard.as_ref().cloned();
    drop(is_guard);

    // Get game type, defaulting to Minecraft for backwards compatibility
    let game_type = game_data.game.clone().unwrap_or(Game::Minecraft);
    let mut players = game_data.players;

    tokio_rt.block_on(async {
        // Resolve in-game names to canonical gamertags
        if let Some(ref id_service) = identity_service {
            // Process any alternative_identity fields from Floodgate-aware mods
            for player in &players {
                let alt = player.get_alternative_identity();

                if let Some(alt_identity) = alt {
                    let name = player.get_name();
                    if let Some(player_id) = id_service
                        .find_player_id_by_gamertag(alt_identity, &game_type)
                        .await
                    {
                        let _ = id_service
                            .create_alias(player_id, name, &game_type, "floodgate")
                            .await;
                    }
                }
            }

            id_service
                .resolve_and_remap_players(&mut players, &game_type)
                .await;
        }

        // Process player registration
        if let Some(registrar) = player_registrar {
            let players_clone = players.clone();
            let game_type_clone = game_type.clone();
            // Fire-and-forget player registration in background task
            // This ensures bvc_update_positions returns immediately without waiting for DB operations
            tokio::spawn(async move {
                registrar
                    .process_players(&players_clone, game_type_clone)
                    .await;
            });
        } else {
            tracing::warn!(
                "FFI: PlayerRegistrarService not available - player registration skipped"
            );
        }

        // Broadcast positions to QUIC clients (this happens immediately)
        position_updater::PositionUpdater::broadcast_positions(players, &webhook_receiver_clone)
            .await;
    });

    0
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_audio_play(
    handle: *mut RuntimeHandle,
    play_json: *const c_char,
) -> *mut c_char {
    ffi_guard!("bvc_audio_play", ptr::null_mut(), {
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return ptr::null_mut();
    }

    if play_json.is_null() {
        FfiError::set_last_error("play_json is null");
        return ptr::null_mut();
    }

    let json_str = match unsafe { CStr::from_ptr(play_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in play_json: {}", e));
            return ptr::null_mut();
        }
    };

    let request: common::request::AudioPlayRequest = match serde_json::from_str(json_str) {
        Ok(r) => r,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to parse play_json: {}", e));
            return ptr::null_mut();
        }
    };

    let handle_ref = unsafe { &*handle };

    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            FfiError::set_last_error("Tokio runtime not available");
            return ptr::null_mut();
        }
    };

    let aps_guard = match handle_ref.audio_playback_service.read() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read audio_playback_service: {}", e));
            return ptr::null_mut();
        }
    };

    let audio_service = match aps_guard.as_ref() {
        Some(s) => s.clone(),
        None => {
            FfiError::set_last_error("Server not started - audio_playback_service not available");
            return ptr::null_mut();
        }
    };
    drop(aps_guard);

    let db_guard = match handle_ref.db_conn.read() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read db_conn: {}", e));
            return ptr::null_mut();
        }
    };

    let db_conn = match db_guard.as_ref() {
        Some(c) => c.clone(),
        None => {
            FfiError::set_last_error("Server not started - db_conn not available");
            return ptr::null_mut();
        }
    };
    drop(db_guard);

    let result = tokio_rt.block_on(async {
        audio_service
            .start_playback(db_conn.as_ref(), request)
            .await
    });

    match result {
        Ok(response) => match serde_json::to_string(&response) {
            Ok(json) => match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(e) => {
                    FfiError::set_last_error(&format!("Failed to create CString: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to serialize response: {}", e));
                ptr::null_mut()
            }
        },
        Err(e) => {
            FfiError::set_last_error(&format!("Audio play failed: {}", e));
            ptr::null_mut()
        }
    }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_audio_stop(
    handle: *mut RuntimeHandle,
    event_id: *const c_char,
) -> c_int {
    ffi_guard!("bvc_audio_stop", -1, {
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return -1;
    }

    if event_id.is_null() {
        FfiError::set_last_error("event_id is null");
        return -1;
    }

    let event_id_str = match unsafe { CStr::from_ptr(event_id) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in event_id: {}", e));
            return -1;
        }
    };

    let handle_ref = unsafe { &*handle };

    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            FfiError::set_last_error("Tokio runtime not available");
            return -1;
        }
    };

    let aps_guard = match handle_ref.audio_playback_service.read() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read audio_playback_service: {}", e));
            return -1;
        }
    };

    let audio_service = match aps_guard.as_ref() {
        Some(s) => s.clone(),
        None => {
            FfiError::set_last_error("Server not started - audio_playback_service not available");
            return -1;
        }
    };
    drop(aps_guard);

    let result = tokio_rt.block_on(async { audio_service.stop_playback(event_id_str).await });

    match result {
        Ok(_) => 0,
        Err(e) => {
            FfiError::set_last_error(&format!("Audio stop failed: {}", e));
            -1
        }
    }
    })
}

/// Submit an in-game control action (embedded mode). Parses a `ClientAction` and
/// routes it: self/preference actions deliver ClientBound to the actor's own
/// connection; group actions mutate `ChannelCollection` + fan `ChannelEvent`.
///
/// `group_code_out`, when non-null, receives a heap string holding the new group's
/// share code after a successful `CreateGroup` (free via `bvc_free_string`) and is
/// set to null for every other action or on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_client_action(
    handle: *mut RuntimeHandle,
    action_json: *const c_char,
    group_code_out: *mut *mut c_char,
) -> c_int {
    ffi_guard!("bvc_client_action", -1, {
    if !group_code_out.is_null() {
        unsafe { *group_code_out = ptr::null_mut() };
    }
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return -1;
    }
    if action_json.is_null() {
        FfiError::set_last_error("action_json is null");
        return -1;
    }

    let json = match unsafe { CStr::from_ptr(action_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in action_json: {}", e));
            return -1;
        }
    };

    let mut action: common::structs::control::ClientAction = match serde_json::from_str(json) {
        Ok(a) => a,
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid ClientAction JSON: {}", e));
            return -1;
        }
    };

    let handle_ref = unsafe { &*handle };

    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            FfiError::set_last_error("Tokio runtime not available");
            return -1;
        }
    };

    let cache_manager = {
        let g = match handle_ref.cache_manager.read() {
            Ok(g) => g,
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read cache_manager: {}", e));
                return -1;
            }
        };
        match g.as_ref() {
            Some(cm) => cm.clone(),
            None => {
                FfiError::set_last_error("Server not started - cache_manager not available");
                return -1;
            }
        }
    };

    // Resolve the in-game name to its canonical gamertag (Floodgate/Java aliases)
    // before routing, matching the position ingress so control actions key on the
    // same identity the voice plane uses.
    let identity_service = {
        let g = match handle_ref.identity_service.read() {
            Ok(g) => g,
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read identity_service: {}", e));
                return -1;
            }
        };
        g.as_ref().cloned()
    };
    if let Some(id_service) = identity_service {
        let resolved =
            tokio_rt.block_on(async { id_service.resolve_name(&action.id, &common::Game::Minecraft).await });
        action.id = resolved;
    }

    let svc = crate::services::ClientActionService::new(
        handle_ref.recording_enabled.load(Ordering::Relaxed),
    );

    if !svc.permits(&action.action) {
        FfiError::set_last_error("this server does not permit recording");
        return -1;
    }

    if action.action.is_group_action() {
        let webhook = {
            let g = match handle_ref.webhook_receiver.read() {
                Ok(g) => g,
                Err(e) => {
                    FfiError::set_last_error(&format!("Failed to read webhook_receiver: {}", e));
                    return -1;
                }
            };
            match g.as_ref() {
                Some(w) => w.clone(),
                None => {
                    FfiError::set_last_error("Server not started - webhook_receiver not available");
                    return -1;
                }
            }
        };
        let channels = cache_manager.get_channel_collection();
        let actor_cn = action.actor_key();
        let result = tokio_rt.block_on(async {
            crate::services::ClientActionService::route_group(
                &action.action,
                &actor_cn,
                &channels,
                &webhook,
            )
            .await
        });
        match result {
            Ok(created) => {
                if let (Some(code), false) = (created, group_code_out.is_null()) {
                    match CString::new(code) {
                        Ok(cstr) => unsafe { *group_code_out = cstr.into_raw() },
                        Err(e) => {
                            FfiError::set_last_error(&format!("Failed to create CString: {}", e));
                            return -1;
                        }
                    }
                }
                0
            }
            Err(e) => {
                FfiError::set_last_error(&format!("route_group failed: {}", e));
                -1
            }
        }
    } else {
        match cache_manager.get_connection_registry() {
            Some(registry) => {
                tokio_rt.block_on(async {
                    svc.route_self_with_echo(
                        &action,
                        &action.actor_key(),
                        &registry,
                        cache_manager.player_state(),
                        cache_manager.preferences(),
                    )
                    .await
                });
                0
            }
            None => {
                FfiError::set_last_error("connection_registry not available");
                -1
            }
        }
    }
    })
}

/// Provision a player (idempotent create) and return a fresh single-use login
/// code that a client can later redeem via `code_login`.
///
/// # Arguments
/// * `handle` - Handle from `bvc_server_create()`
/// * `gamertag` - Player gamertag to provision
/// * `game` - Game type (e.g. "minecraft", "hytale")
/// * `ttl_secs` - Lifetime of the generated code in seconds
///
/// # Returns
/// * Pointer to a heap-allocated login code string on success (free via `bvc_free_string`)
/// * NULL on error (call `bvc_get_last_error()` for details)
///
/// # Safety
/// * `handle` must be a valid pointer from `bvc_server_create()`
/// * `gamertag` and `game` must be valid null-terminated UTF-8 strings
/// * Server must be running (after `bvc_server_start()` has been called)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_provision_login_code(
    handle: *mut RuntimeHandle,
    gamertag: *const c_char,
    game: *const c_char,
    ttl_secs: u32,
) -> *mut c_char {
    ffi_guard!("bvc_provision_login_code", ptr::null_mut(), {
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return ptr::null_mut();
    }

    if gamertag.is_null() {
        FfiError::set_last_error("gamertag is null");
        return ptr::null_mut();
    }

    if game.is_null() {
        FfiError::set_last_error("game is null");
        return ptr::null_mut();
    }

    let gamertag_str = match unsafe { CStr::from_ptr(gamertag) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in gamertag: {}", e));
            return ptr::null_mut();
        }
    };

    let game_str = match unsafe { CStr::from_ptr(game) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in game: {}", e));
            return ptr::null_mut();
        }
    };

    let game_type = match game_str {
        "minecraft" => Game::Minecraft,
        "hytale" => Game::Hytale,
        other => {
            FfiError::set_last_error(&format!("Invalid game '{}'", other));
            return ptr::null_mut();
        }
    };

    let handle_ref = unsafe { &*handle };

    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            FfiError::set_last_error("Tokio runtime not available");
            return ptr::null_mut();
        }
    };

    let pr_guard = match handle_ref.player_registrar.read() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read player_registrar: {}", e));
            return ptr::null_mut();
        }
    };

    let registrar = match pr_guard.as_ref() {
        Some(r) => r.clone(),
        None => {
            FfiError::set_last_error("Server not started - player_registrar not available");
            return ptr::null_mut();
        }
    };
    drop(pr_guard);

    let db_guard = match handle_ref.db_conn.read() {
        Ok(g) => g,
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read db_conn: {}", e));
            return ptr::null_mut();
        }
    };

    let db_conn = match db_guard.as_ref() {
        Some(c) => c.clone(),
        None => {
            FfiError::set_last_error("Server not started - db_conn not available");
            return ptr::null_mut();
        }
    };
    drop(db_guard);

    let result = tokio_rt.block_on(async {
        let player = registrar
            .create_player(gamertag_str, &game_type, None)
            .await?;
        // FFI-minted codes are single-use (ephemeral), matching prior behavior.
        AuthCodeService::generate_code(db_conn.as_ref(), player.id, ttl_secs as u64, true).await
    });

    match result {
        Ok(code) => match CString::new(code) {
            Ok(cstr) => cstr.into_raw(),
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to create CString: {}", e));
                ptr::null_mut()
            }
        },
        Err(e) => {
            FfiError::set_last_error(&format!("Provision login code failed: {}", e));
            ptr::null_mut()
        }
    }
    })
}

/// Mint a WebSocket ticket for a gamertag, without an mTLS round trip.
///
/// The HTTP route that issues these exchanges an mTLS identity for a ticket, and the
/// response it hands back is ncryptf-encrypted. An in-process harness would have to
/// reimplement both to watch the position feed, which is a lot of machinery to test a
/// socket — so the same provisioning seam `bvc_provision_login_code` uses is offered here.
///
/// The ticket is single-use and short-lived exactly as an HTTP-issued one is; nothing about
/// its lifetime or its redemption differs.
///
/// # Returns
/// * Pointer to a heap-allocated ticket string on success (free via `bvc_free_string`)
/// * NULL on error (call `bvc_get_last_error()` for details)
///
/// # Safety
/// * `handle` must be a valid pointer from `bvc_server_create()`
/// * `gamertag` must be a valid null-terminated UTF-8 string
/// * Server must be running (after `bvc_server_start()` has been called)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_provision_websocket_ticket(
    handle: *mut RuntimeHandle,
    gamertag: *const c_char,
    game: *const c_char,
) -> *mut c_char {
    ffi_guard!("bvc_provision_websocket_ticket", ptr::null_mut(), {
    if handle.is_null() {
        FfiError::set_last_error("handle is null");
        return ptr::null_mut();
    }

    if gamertag.is_null() || game.is_null() {
        FfiError::set_last_error("gamertag or game is null");
        return ptr::null_mut();
    }

    let gamertag_str = match unsafe { CStr::from_ptr(gamertag) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in gamertag: {}", e));
            return ptr::null_mut();
        }
    };

    let game_type = match unsafe { CStr::from_ptr(game) }.to_str() {
        Ok("minecraft") => Game::Minecraft,
        Ok("hytale") => Game::Hytale,
        Ok(other) => {
            FfiError::set_last_error(&format!("Invalid game '{}'", other));
            return ptr::null_mut();
        }
        Err(e) => {
            FfiError::set_last_error(&format!("Invalid UTF-8 in game: {}", e));
            return ptr::null_mut();
        }
    };

    let handle_ref = unsafe { &*handle };

    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            FfiError::set_last_error("Tokio runtime not available");
            return ptr::null_mut();
        }
    };

    let cache_manager = match handle_ref.cache_manager.read() {
        Ok(guard) => match guard.as_ref() {
            Some(cm) => cm.clone(),
            None => {
                FfiError::set_last_error("Server not started - cache_manager not available");
                return ptr::null_mut();
            }
        },
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to read cache_manager: {}", e));
            return ptr::null_mut();
        }
    };

    let ticket = tokio_rt.block_on(async {
        cache_manager
            .websocket_tickets()
            .issue(crate::stream::quic::TicketIdentity {
                gamertag: gamertag_str,
                game: game_type,
            })
            .await
    });

    match CString::new(ticket) {
        Ok(cstr) => cstr.into_raw(),
        Err(e) => {
            FfiError::set_last_error(&format!("Failed to create CString: {}", e));
            ptr::null_mut()
        }
    }
    })
}

/// Register the embedded mod as this world's chat channel.
///
/// The embedded mod shares this process, so it drives chat through calls rather than dialling
/// a socket back into its own address space. It registers into the same `ChatSocketRegistry`
/// the WebSocket route uses, which is what lets `on_app_send`, availability, the worlds route
/// and the QUIC fan-out all work unchanged — the FFI is a transport, not a second
/// implementation of chat.
///
/// `hello_json` is a `ChatFrame::Hello`, the same shape the socket sends.
///
/// # Returns
/// 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_chat_register(
    handle: *mut RuntimeHandle,
    hello_json: *const c_char,
) -> c_int {
    ffi_guard!("bvc_chat_register", -1, {
        if handle.is_null() || hello_json.is_null() {
            FfiError::set_last_error("handle or hello_json is null");
            return -1;
        }

        let json = match unsafe { CStr::from_ptr(hello_json) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                FfiError::set_last_error(&format!("Invalid UTF-8 in hello_json: {}", e));
                return -1;
            }
        };

        let frame: common::structs::chat::ChatFrame = match serde_json::from_str(json) {
            Ok(f) => f,
            Err(e) => {
                FfiError::set_last_error(&format!("Invalid ChatFrame JSON: {}", e));
                return -1;
            }
        };

        let common::structs::chat::ChatFrame::Hello {
            world,
            world_name,
            worlds,
            ..
        } = frame
        else {
            FfiError::set_last_error("bvc_chat_register expects a hello frame");
            return -1;
        };

        let handle_ref = unsafe { &*handle };

        let service = match handle_ref.chat_service.read() {
            Ok(g) => match g.as_ref() {
                Some(s) => s.clone(),
                None => {
                    FfiError::set_last_error("Server not started - chat service not available");
                    return -1;
                }
            },
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read chat_service: {}", e));
                return -1;
            }
        };

        // The canonical id first, then any extra ids the same room spans, deduplicated.
        let mut keys = vec![world];
        for extra in worlds {
            if !keys.contains(&extra) {
                keys.push(extra);
            }
        }

        // Bounded for the same reason the socket queue is: chat that cannot be drained is
        // dropped rather than buffered, because a backlog delivered late lands stale lines in
        // a conversation that has already moved on.
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(64);

        let socket_id = service.next_socket_id();
        for previous in service.register_room(socket_id, &keys, world_name, tx) {
            drop(previous);
        }

        if let Ok(mut slot) = handle_ref.chat_outbound.lock() {
            *slot = Some(rx);
        }
        if let Ok(mut rooms) = handle_ref.chat_rooms.lock() {
            *rooms = keys;
        }
        handle_ref.chat_socket_id.store(socket_id, Ordering::SeqCst);

        0
    })
}

/// This server's own peerlink, for a bridge running beside it.
///
/// Minted from the live endpoint, so it carries an address a bridge on the same host
/// can dial. That is why it cannot be derived from the key file alone the way a
/// bridge's own link can: the bridge dials the server, so it needs somewhere to go.
///
/// # Returns
/// A newly allocated string the caller frees with `bvc_free_string`, or null when
/// this server declares no peers and therefore binds no peer endpoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_relay_peerlink(handle: *mut RuntimeHandle) -> *mut c_char {
    ffi_guard!("bvc_relay_peerlink", ptr::null_mut(), {
        if handle.is_null() {
            FfiError::set_last_error("handle is null");
            return ptr::null_mut();
        }

        let handle_ref = unsafe { &*handle };

        let cache_manager = match handle_ref.cache_manager.read() {
            Ok(guard) => match guard.as_ref() {
                Some(cm) => cm.clone(),
                None => {
                    FfiError::set_last_error("Server not started - cache_manager not available");
                    return ptr::null_mut();
                }
            },
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read cache_manager: {}", e));
                return ptr::null_mut();
            }
        };

        let Some(registry) = cache_manager.get_connection_registry() else {
            FfiError::set_last_error("connection registry not available");
            return ptr::null_mut();
        };

        let Some(plane) = registry.peer_plane() else {
            FfiError::set_last_error("this server declares no peers");
            return ptr::null_mut();
        };

        let tokio_rt = match &handle_ref.tokio_runtime {
            Some(rt) => rt,
            None => {
                FfiError::set_last_error("tokio runtime not available");
                return ptr::null_mut();
            }
        };

        match tokio_rt.block_on(plane.endpoint().ticket()) {
            Ok(link) => match CString::new(link) {
                Ok(s) => s.into_raw(),
                Err(e) => {
                    FfiError::set_last_error(&format!("peerlink contained a nul: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                FfiError::set_last_error(&format!("minting a peerlink failed: {}", e));
                ptr::null_mut()
            }
        }
    })
}

/// Whether a player has a live voice connection to this server.
///
/// `identity` is the membership key, which is what the connection registry indexes.
///
/// The SVC bridge asks so it can leave those players out of its injection: a Java
/// player running both Simple Voice Chat and the BVC desktop client would otherwise
/// hear every remote speaker twice.
///
/// # Returns
/// 1 when live, 0 when not, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_has_live_client(
    handle: *mut RuntimeHandle,
    identity: *const c_char,
) -> c_int {
    ffi_guard!("bvc_has_live_client", -1, {
        if handle.is_null() || identity.is_null() {
            FfiError::set_last_error("handle or identity is null");
            return -1;
        }

        let identity = match unsafe { CStr::from_ptr(identity) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                FfiError::set_last_error(&format!("Invalid UTF-8 in identity: {}", e));
                return -1;
            }
        };

        let handle_ref = unsafe { &*handle };

        let cache_manager = match handle_ref.cache_manager.read() {
            Ok(guard) => match guard.as_ref() {
                Some(cm) => cm.clone(),
                None => {
                    FfiError::set_last_error("Server not started - cache_manager not available");
                    return -1;
                }
            },
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read cache_manager: {}", e));
                return -1;
            }
        };

        let Some(registry) = cache_manager.get_connection_registry() else {
            FfiError::set_last_error("connection registry not available");
            return -1;
        };

        i32::from(registry.has_live_client(identity))
    })
}

/// Report whether this host could fetch and write a native library.
///
/// `report_json` is a HostCapability: variant, platform, mod_version, fetch, write.
/// Anything outside the known vocabulary is refused rather than forwarded.
///
/// The external mod reports the same fact over HTTP. Embedded has no socket to the
/// server it is running in-process, so without this the embedded population — the
/// one most likely to differ — would go unmeasured.
///
/// # Returns
/// 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_host_capability(
    handle: *mut RuntimeHandle,
    report_json: *const c_char,
) -> c_int {
    ffi_guard!("bvc_host_capability", -1, {
        if handle.is_null() || report_json.is_null() {
            FfiError::set_last_error("handle or report_json is null");
            return -1;
        }

        let json = match unsafe { CStr::from_ptr(report_json) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                FfiError::set_last_error(&format!("Invalid UTF-8 in report_json: {}", e));
                return -1;
            }
        };

        let report = match crate::services::HostCapability::parse(json) {
            Ok(report) => report,
            Err(e) => {
                FfiError::set_last_error(&format!("Invalid HostCapability JSON: {}", e));
                return -1;
            }
        };

        let handle_ref = unsafe { &*handle };

        let metrics = match handle_ref.metrics.read() {
            Ok(guard) => match guard.as_ref() {
                Some(metrics) => metrics.clone(),
                None => {
                    FfiError::set_last_error("metrics service is not available");
                    return -1;
                }
            },
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read metrics: {}", e));
                return -1;
            }
        };

        metrics.record_host_capability(report);
        0
    })
}

/// Report a line a player typed in game.
///
/// `chat_json` is a `ChatFrame::Chat`.
///
/// # Returns
/// 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_chat_report(
    handle: *mut RuntimeHandle,
    chat_json: *const c_char,
) -> c_int {
    ffi_guard!("bvc_chat_report", -1, {
        if handle.is_null() || chat_json.is_null() {
            FfiError::set_last_error("handle or chat_json is null");
            return -1;
        }

        let json = match unsafe { CStr::from_ptr(chat_json) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                FfiError::set_last_error(&format!("Invalid UTF-8 in chat_json: {}", e));
                return -1;
            }
        };

        let frame: common::structs::chat::ChatFrame = match serde_json::from_str(json) {
            Ok(f) => f,
            Err(e) => {
                FfiError::set_last_error(&format!("Invalid ChatFrame JSON: {}", e));
                return -1;
            }
        };

        // Both directions of "something happened in the world": a person speaking, or the
        // server speaking. Embedded has to carry events too, or deaths and joins appear on
        // Bedrock and vanish on Java.
        let reported = match frame {
            common::structs::chat::ChatFrame::Chat { author, text } => Some((Some(author), text)),
            common::structs::chat::ChatFrame::Event { text } => Some((None, text)),
            _ => None,
        };

        let Some((author, text)) = reported else {
            FfiError::set_last_error("bvc_chat_report expects a chat or event frame");
            return -1;
        };

        let handle_ref = unsafe { &*handle };

        let tokio_rt = match &handle_ref.tokio_runtime {
            Some(rt) => rt,
            None => {
                FfiError::set_last_error("Tokio runtime not available");
                return -1;
            }
        };

        let service = match handle_ref.chat_service.read() {
            Ok(g) => match g.as_ref() {
                Some(s) => s.clone(),
                None => {
                    FfiError::set_last_error("Server not started - chat service not available");
                    return -1;
                }
            },
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read chat_service: {}", e));
                return -1;
            }
        };

        let rooms = match handle_ref.chat_rooms.lock() {
            Ok(r) => r.clone(),
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read chat_rooms: {}", e));
                return -1;
            }
        };

        if rooms.is_empty() {
            FfiError::set_last_error("bvc_chat_register has not been called");
            return -1;
        }

        tokio_rt.block_on(async move {
            match author {
                Some(author) => service.on_game_chat(&rooms, author, text).await,
                None => service.on_game_event(&rooms, text).await,
            }
        });

        0
    })
}

/// Take every `say` frame waiting for the embedded mod to broadcast.
///
/// Polled from the tick the mod already runs. Returns a JSON array — `[]` when there is
/// nothing — which the caller must release with `bvc_free_string`.
///
/// A pull rather than a callback because the FFI has no callback mechanism at all, and adding
/// one would mean holding a function pointer across the JNA boundary for the life of the
/// process.
///
/// # Returns
/// Pointer to a JSON array, or null on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_chat_drain(handle: *mut RuntimeHandle) -> *mut c_char {
    ffi_guard!("bvc_chat_drain", ptr::null_mut(), {
        if handle.is_null() {
            FfiError::set_last_error("handle is null");
            return ptr::null_mut();
        }

        let handle_ref = unsafe { &*handle };

        let mut frames: Vec<serde_json::Value> = Vec::new();

        if let Ok(mut slot) = handle_ref.chat_outbound.lock() {
            if let Some(rx) = slot.as_mut() {
                // Non-blocking: this runs on the game's tick thread, which must never park.
                while let Ok(body) = rx.try_recv() {
                    match serde_json::from_str::<serde_json::Value>(&body) {
                        Ok(v) => frames.push(v),
                        Err(e) => tracing::warn!("undecodable outbound chat frame: {}", e),
                    }
                }
            }
        }

        let body = match serde_json::to_string(&frames) {
            Ok(b) => b,
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to encode chat frames: {}", e));
                return ptr::null_mut();
            }
        };

        match CString::new(body) {
            Ok(c) => c.into_raw(),
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to build C string: {}", e));
                ptr::null_mut()
            }
        }
    })
}

/// Release every chat room the embedded mod registered.
///
/// # Returns
/// 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_chat_unregister(handle: *mut RuntimeHandle) -> c_int {
    ffi_guard!("bvc_chat_unregister", -1, {
        if handle.is_null() {
            FfiError::set_last_error("handle is null");
            return -1;
        }

        let handle_ref = unsafe { &*handle };

        let service = match handle_ref.chat_service.read() {
            Ok(g) => g.as_ref().cloned(),
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to read chat_service: {}", e));
                return -1;
            }
        };

        if let (Some(service), Ok(mut rooms)) = (service, handle_ref.chat_rooms.lock()) {
            let socket_id = handle_ref.chat_socket_id.load(Ordering::SeqCst);
            for world in rooms.iter() {
                service.unregister(world, socket_id);
            }
            rooms.clear();
        }

        if let Ok(mut slot) = handle_ref.chat_outbound.lock() {
            *slot = None;
        }

        0
    })
}

/// Return the configuration the server resolved: the embedder's JSON with serde
/// defaults and `BVC_*` overrides applied.
///
/// The embedded mod needs values it never set — the HTTP port and TLS name its
/// own chat endpoint dials — and those are decided here rather than in the mod.
///
/// # Returns
/// * Pointer to a heap-allocated JSON string (free via `bvc_free_string`)
/// * NULL on error
///
/// # Safety
/// * `handle` must be a valid pointer from `bvc_server_create()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bvc_config_effective(handle: *mut RuntimeHandle) -> *mut c_char {
    ffi_guard!("bvc_config_effective", ptr::null_mut(), {
        if handle.is_null() {
            FfiError::set_last_error("handle is null");
            return ptr::null_mut();
        }

        let handle_ref = unsafe { &*handle };

        match CString::new(handle_ref.resolved_config.clone()) {
            Ok(cstr) => cstr.into_raw(),
            Err(e) => {
                FfiError::set_last_error(&format!("Failed to create CString: {}", e));
                ptr::null_mut()
            }
        }
    })
}
