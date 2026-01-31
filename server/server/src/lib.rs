//! BVC Server Library
//!
//! This library provides the core BVC server functionality that can be used
//! either as a standalone server or embedded into other applications via FFI.

/// The version of the BVC server, embedded at compile time from Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

extern crate common;

#[macro_use]
extern crate rocket;

// Internal modules (used by runtime)
pub(crate) mod config;
pub(crate) mod rs;
pub mod services;
pub(crate) mod stream;

// Public modules
pub mod ffi;
pub mod runtime;

// Re-exports for public API
pub use config::{
    ApplicationConfig, ApplicationConfigDatabase, ApplicationConfigLogger,
    ApplicationConfigMinecraft, ApplicationConfigServer, ApplicationConfigServerTLS,
    ApplicationConfigVoice,
};
pub use runtime::{RuntimeState, ServerRuntime};

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Initialize the crypto provider (rustls with aws-lc-rs).
/// Must be called before creating any ServerRuntime instances.
/// Safe to call multiple times - will return Ok if already initialized.
pub fn init_crypto_provider() -> Result<(), &'static str> {
    common::s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| "Crypto provider already installed")
        .or(Ok(()))
}

/// Initialize Windows timer resolution for high-precision audio timing.
/// Only has effect on Windows; no-op on other platforms.
#[cfg(target_os = "windows")]
pub fn init_windows_timer() {
    windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
    windows_targets::link!("ntdll.dll" "system" fn NtQueryTimerResolution(
        minimumresolution: *mut u32,
        maximumresolution: *mut u32,
        currentresolution: *mut u32,
    ) -> i32);

    unsafe {
        // Check current timer resolution before setting
        let mut min_res = 0u32;
        let mut max_res = 0u32;
        let mut current_res = 0u32;
        NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
        let current_ms = current_res as f64 / 10_000.0;
        tracing::info!("Current Windows timer resolution: {:.2}ms", current_ms);

        // Set to 1ms
        timeBeginPeriod(1);

        // Verify the change
        NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
        let new_ms = current_res as f64 / 10_000.0;
        tracing::info!(
            "Set Windows timer resolution to 1ms (actual: {:.2}ms)",
            new_ms
        );

        if new_ms > 2.0 {
            tracing::warn!(
                "Timer resolution is degraded ({:.2}ms). This may cause audio jitter!",
                new_ms
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn init_windows_timer() {
    // No-op on non-Windows platforms
}