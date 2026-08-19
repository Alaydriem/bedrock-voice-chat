use std::sync::Arc;

use tauri::State;

use crate::spike::{LoopbackProbe, ProbePorts, ProbeStats, PushProtocol};

/// THROWAWAY. Bind the loopback probe listeners and report their ports.
#[tauri::command]
pub(crate) async fn spike_probe_start(
    probe: State<'_, Arc<LoopbackProbe>>,
) -> Result<ProbePorts, String> {
    probe.start().await.map_err(|e| e.to_string())
}

/// THROWAWAY. What the listening side observed.
#[tauri::command]
pub(crate) async fn spike_probe_stats(
    probe: State<'_, Arc<LoopbackProbe>>,
) -> Result<ProbeStats, String> {
    Ok(probe.stats())
}

/// THROWAWAY. How many custom-scheme push requests this process has answered.
#[tauri::command]
pub(crate) async fn spike_push_served(
    push: State<'_, Arc<PushProtocol>>,
) -> Result<u32, String> {
    Ok(push.served())
}
