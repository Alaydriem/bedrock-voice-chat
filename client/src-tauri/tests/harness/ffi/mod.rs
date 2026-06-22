use std::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::sync::Arc;

use libloading::{Library, Symbol};

mod send_handle;

pub use send_handle::SendHandle;

/// Opaque server handle pointer returned by `bvc_server_create`. The library
/// owns the pointee; the harness only moves the raw pointer between threads.
pub type RuntimeHandlePtr = *mut std::ffi::c_void;

/// Resolved `bvc_*` symbols from the server cdylib, kept alive alongside the
/// `Library` they borrow from. Symbols are transmuted to `'static` fn pointers
/// whose validity is tied to `_lib`; `ServerLibrary` holds both so the contract
/// is upheld for the library's lifetime.
pub struct ServerLibrary {
    _lib: Library,
    pub version: unsafe extern "C" fn() -> *const c_char,
    pub create: unsafe extern "C" fn(*const c_char) -> RuntimeHandlePtr,
    pub start: unsafe extern "C" fn(RuntimeHandlePtr) -> c_int,
    pub stop: unsafe extern "C" fn(RuntimeHandlePtr) -> c_int,
    pub destroy: unsafe extern "C" fn(RuntimeHandlePtr) -> c_int,
    pub last_error: unsafe extern "C" fn() -> *const c_char,
    pub update_positions: unsafe extern "C" fn(RuntimeHandlePtr, *const c_char) -> c_int,
    pub audio_play: unsafe extern "C" fn(RuntimeHandlePtr, *const c_char) -> *mut c_char,
    pub audio_stop: unsafe extern "C" fn(RuntimeHandlePtr, *const c_char) -> c_int,
    pub provision_login_code:
        unsafe extern "C" fn(RuntimeHandlePtr, *const c_char, *const c_char, u32) -> *mut c_char,
    pub free_string: unsafe extern "C" fn(*mut c_char),
}

impl ServerLibrary {
    /// Platform-specific cdylib file name produced by `bvc_server_lib`.
    fn lib_file_name() -> &'static str {
        #[cfg(target_os = "windows")]
        {
            "bvc_server_lib.dll"
        }
        #[cfg(target_os = "macos")]
        {
            "libbvc_server_lib.dylib"
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            "libbvc_server_lib.so"
        }
    }

    /// The server workspace's debug target directory, relative to this crate's
    /// manifest (`client/src-tauri`). The server is a SEPARATE Cargo workspace
    /// rooted at `server/`, so its artifacts land in `server/target`, not the
    /// root workspace's `target`.
    pub fn default_lib_path() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir)
            .join("..")
            .join("..")
            .join("server")
            .join("target")
            .join("debug")
            .join(Self::lib_file_name())
    }

    /// Load the cdylib at `path` and resolve every `bvc_*` symbol the harness
    /// uses. The server cdylib must be built first:
    /// `cargo build -p bedrock-voice-chat-server` from the `server/` workspace.
    pub fn load(path: &std::path::Path) -> Result<Arc<Self>, String> {
        if !path.exists() {
            return Err(format!(
                "server cdylib not found at {}; build it first with \
                 `cargo build -p bedrock-voice-chat-server` in the server/ workspace",
                path.display()
            ));
        }

        unsafe {
            let lib = Library::new(path).map_err(|e| format!("loading {}: {e}", path.display()))?;

            let version =
                Self::sym::<unsafe extern "C" fn() -> *const c_char>(&lib, b"bvc_version")?;
            let create = Self::sym::<unsafe extern "C" fn(*const c_char) -> RuntimeHandlePtr>(
                &lib,
                b"bvc_server_create",
            )?;
            let start = Self::sym::<unsafe extern "C" fn(RuntimeHandlePtr) -> c_int>(
                &lib,
                b"bvc_server_start",
            )?;
            let stop = Self::sym::<unsafe extern "C" fn(RuntimeHandlePtr) -> c_int>(
                &lib,
                b"bvc_server_stop",
            )?;
            let destroy = Self::sym::<unsafe extern "C" fn(RuntimeHandlePtr) -> c_int>(
                &lib,
                b"bvc_server_destroy",
            )?;
            let last_error =
                Self::sym::<unsafe extern "C" fn() -> *const c_char>(&lib, b"bvc_get_last_error")?;
            let update_positions = Self::sym::<
                unsafe extern "C" fn(RuntimeHandlePtr, *const c_char) -> c_int,
            >(&lib, b"bvc_update_positions")?;
            let audio_play = Self::sym::<
                unsafe extern "C" fn(RuntimeHandlePtr, *const c_char) -> *mut c_char,
            >(&lib, b"bvc_audio_play")?;
            let audio_stop = Self::sym::<
                unsafe extern "C" fn(RuntimeHandlePtr, *const c_char) -> c_int,
            >(&lib, b"bvc_audio_stop")?;
            let provision_login_code = Self::sym::<
                unsafe extern "C" fn(
                    RuntimeHandlePtr,
                    *const c_char,
                    *const c_char,
                    u32,
                ) -> *mut c_char,
            >(&lib, b"bvc_provision_login_code")?;
            let free_string =
                Self::sym::<unsafe extern "C" fn(*mut c_char)>(&lib, b"bvc_free_string")?;

            Ok(Arc::new(Self {
                _lib: lib,
                version,
                create,
                start,
                stop,
                destroy,
                last_error,
                update_positions,
                audio_play,
                audio_stop,
                provision_login_code,
                free_string,
            }))
        }
    }

    /// Resolve one symbol and detach its lifetime from the borrow of `lib`. Safe
    /// because the resulting fn pointer is stored next to the owning `Library`
    /// in `ServerLibrary` and never outlives it.
    unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> Result<T, String> {
        let symbol: Symbol<T> = unsafe {
            lib.get(name)
                .map_err(|e| format!("symbol {}: {e}", String::from_utf8_lossy(name)))?
        };
        Ok(unsafe { *symbol.into_raw() })
    }
}
