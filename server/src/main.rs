extern crate common;
use tokio;

mod commands;
mod config;
mod rs;
mod stream;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[macro_use]
extern crate rocket;

#[tokio::main]
async fn main() {
    // Set Windows timer resolution for high-precision audio timing
    #[cfg(target_os = "windows")]
    {
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
            tracing::error!("Current Windows timer resolution: {:.2}ms", current_ms);
            
            // Set to 1ms
            timeBeginPeriod(1);
            
            // Verify the change
            NtQueryTimerResolution(&mut min_res, &mut max_res, &mut current_res);
            let new_ms = current_res as f64 / 10_000.0;
            tracing::error!("Set Windows timer resolution to 1ms (actual: {:.2}ms)", new_ms);
            
            if new_ms > 2.0 {
                tracing::error!("WARNING: Timer resolution is degraded ({:.2}ms). This will cause audio jitter!", new_ms);
                tracing::error!("Try closing other applications or restarting Windows.");
                tracing::error!("Consider running BVC Server on Linux for best performance.");
            }
        }
    }

    _ = s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default();
    let _app = commands::launch().await;
}
