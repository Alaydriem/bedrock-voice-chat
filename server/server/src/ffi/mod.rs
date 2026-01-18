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

use crate::config::ApplicationConfig;
use crate::runtime::{position_updater, ServerRuntime};
use crate::stream::quic::WebhookReceiver;

use std::ffi::{c_char, c_int, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// Opaque handle to a server runtime instance
pub struct RuntimeHandle {
    runtime: Mutex<Option<ServerRuntime>>,
    tokio_runtime: Option<tokio::runtime::Runtime>,
    /// Shutdown flag - accessible without locking runtime mutex
    shutdown_flag: Arc<AtomicBool>,
    /// Webhook receiver for position updates - accessible without locking runtime mutex
    webhook_receiver: Arc<RwLock<Option<WebhookReceiver>>>,
}

// Thread-local storage for last error message
thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

fn set_last_error(msg: &str) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(msg).ok();
    });
}

/// Initialize the crypto provider. Must be called before creating any servers.
/// Safe to call multiple times.
///
/// Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn bvc_init() -> c_int {
    match crate::init_crypto_provider() {
        Ok(_) => 0,
        Err(e) => {
            set_last_error(e);
            -1
        }
    }
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
    if config_json.is_null() {
        set_last_error("config_json is null");
        return ptr::null_mut();
    }

    let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8 in config_json: {}", e));
            return ptr::null_mut();
        }
    };

    let config: ApplicationConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            set_last_error(&format!("Failed to parse config JSON: {}", e));
            return ptr::null_mut();
        }
    };

    let runtime = match ServerRuntime::new(config) {
        Ok(r) => r,
        Err(e) => {
            set_last_error(&format!("Failed to create runtime: {}", e));
            return ptr::null_mut();
        }
    };

    // Extract Arc clones BEFORE putting runtime in Mutex
    // This allows stop() and update_positions() to work without locking the runtime
    let shutdown_flag = runtime.shutdown_flag();
    let webhook_receiver = runtime.get_webhook_receiver();

    // Create a tokio runtime for the server
    let tokio_runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            set_last_error(&format!("Failed to create tokio runtime: {}", e));
            return ptr::null_mut();
        }
    };

    let handle = Box::new(RuntimeHandle {
        runtime: Mutex::new(Some(runtime)),
        tokio_runtime: Some(tokio_runtime),
        shutdown_flag,
        webhook_receiver,
    });

    Box::into_raw(handle)
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
    if handle.is_null() {
        set_last_error("handle is null");
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    // Get the tokio runtime
    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            set_last_error("Tokio runtime not available");
            return -1;
        }
    };

    // Get mutable access to the server runtime
    let mut runtime_guard = match handle_ref.runtime.lock() {
        Ok(g) => g,
        Err(e) => {
            set_last_error(&format!("Failed to lock runtime: {}", e));
            return -1;
        }
    };

    let runtime = match runtime_guard.as_mut() {
        Some(r) => r,
        None => {
            set_last_error("Runtime already consumed or not initialized");
            return -1;
        }
    };

    // Run the server on the tokio runtime (blocks until shutdown)
    let result = tokio_rt.block_on(async { runtime.start_async().await });

    match result {
        Ok(_) => 0,
        Err(e) => {
            set_last_error(&format!("Server error: {}", e));
            -1
        }
    }
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
    if handle.is_null() {
        set_last_error("handle is null");
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    // Use the shutdown flag directly - no mutex lock required
    // This avoids deadlock since start() holds the runtime mutex
    handle_ref.shutdown_flag.store(true, Ordering::SeqCst);
    0
}

/// Destroy the server handle and free all resources.
///
/// Call this after `bvc_server_start()` returns.
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
    if handle.is_null() {
        set_last_error("handle is null");
        return -1;
    }

    // Take ownership and drop
    let _ = unsafe { Box::from_raw(handle) };
    0
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
    LAST_ERROR.with(|e| {
        e.borrow()
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null())
    })
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
    if handle.is_null() {
        set_last_error("handle is null");
        return -1;
    }

    if game_data_json.is_null() {
        set_last_error("game_data_json is null");
        return -1;
    }

    let json_str = match unsafe { CStr::from_ptr(game_data_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8 in game_data_json: {}", e));
            return -1;
        }
    };

    // Parse the GameDataCollection JSON
    let game_data: common::GameDataCollection = match serde_json::from_str(json_str) {
        Ok(data) => data,
        Err(e) => {
            set_last_error(&format!("Failed to parse game_data JSON: {}", e));
            return -1;
        }
    };

    let handle_ref = unsafe { &*handle };

    // Get the tokio runtime
    let tokio_rt = match &handle_ref.tokio_runtime {
        Some(rt) => rt,
        None => {
            set_last_error("Tokio runtime not available");
            return -1;
        }
    };

    // Use the webhook_receiver directly - no mutex lock required
    // This avoids deadlock since start() holds the runtime mutex
    let wr_guard = match handle_ref.webhook_receiver.read() {
        Ok(g) => g,
        Err(e) => {
            set_last_error(&format!("Failed to read webhook_receiver: {}", e));
            return -1;
        }
    };

    let webhook_receiver = match wr_guard.as_ref() {
        Some(wr) => wr,
        None => {
            set_last_error("Server not started - webhook_receiver not available");
            return -1;
        }
    };

    // Send position update (run async operation on tokio runtime)
    // Clone the webhook_receiver reference to satisfy borrow checker
    let webhook_receiver_clone = webhook_receiver.clone();
    drop(wr_guard);  // Release read lock before blocking

    tokio_rt.block_on(async {
        position_updater::broadcast_positions(game_data.players, &webhook_receiver_clone).await;
    });

    0
}
