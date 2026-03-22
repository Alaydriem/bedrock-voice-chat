pub const VERSION: &str = env!("CARGO_PKG_VERSION");

extern crate common;

#[macro_use]
extern crate rocket;

pub(crate) mod config;
pub mod http;
pub mod services;
pub(crate) mod stream;

pub mod ffi;
pub mod runtime;

pub use config::ApplicationConfig;
pub use runtime::{RuntimeState, ServerRuntime};

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub struct BvcServer;

impl BvcServer {
    pub fn new(config: ApplicationConfig) -> Result<ServerRuntime, anyhow::Error> {
        Self::init_platform();
        ServerRuntime::new(config)
    }

    fn init_platform() {
        let _ = common::s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
            .install_default();

        #[cfg(target_os = "windows")]
        {
            windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
            windows_targets::link!("ntdll.dll" "system" fn NtQueryTimerResolution(
                minimumresolution: *mut u32,
                maximumresolution: *mut u32,
                currentresolution: *mut u32,
            ) -> i32);

            unsafe {
                let mut min_res = 0u32;
                let mut max_res = 0u32;
                let mut current_res = 0u32;
                NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
                let current_ms = current_res as f64 / 10_000.0;
                tracing::info!("Current Windows timer resolution: {:.2}ms", current_ms);

                timeBeginPeriod(1);

                NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
                let new_ms = current_res as f64 / 10_000.0;
                tracing::info!("Set Windows timer resolution to 1ms (actual: {:.2}ms)", new_ms);

                if new_ms > 2.0 {
                    tracing::warn!("Timer resolution is degraded ({:.2}ms). This may cause audio jitter!", new_ms);
                }
            }
        }
    }
}
