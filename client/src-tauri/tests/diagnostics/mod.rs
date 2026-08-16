mod adaptive_inertness;
mod ring;
mod route;
// Drives `LinkDiagnosticsService::tick_for_test` and `DeviceInfo::set_noise_gate_enabled`,
// both `#[cfg(any(test, feature = "e2e"))]`, so this module compiles solely in the
// e2e configuration.
#[cfg(feature = "e2e")]
mod service;
mod stats;
