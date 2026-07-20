/// Shared state the stdout-reader thread writes and the orchestrator reads.
/// `ready`/`connected` latch on first sight of the matching `OutMsg`; `captured`
/// accumulates every `CapturedPcm` chunk in arrival order; `stats` holds the
/// most recently received transport-counter snapshot from `OutMsg::Stats`.
#[derive(Default)]
pub(super) struct SharedState {
    pub(super) ready: bool,
    pub(super) connected: bool,
    pub(super) disconnected: bool,
    // Server-assigned channel id reported by OutMsg::ChannelJoined after connect.
    pub(super) channel_id: Option<String>,
    // Name of the most recently completed channel op (OutMsg::ChannelOpDone),
    // cleared before each op so the orchestrator can await a fresh completion.
    pub(super) last_channel_op: Option<String>,
    // Server-assigned (audio_file_id, duration_ms) from the last UploadAudio.
    pub(super) last_upload: Option<(String, u32)>,
    // Local listen port reported by OutMsg::ProxyStarted after a StartProxy command.
    pub(super) proxy_listen: Option<u16>,
    pub(super) captured: Vec<f32>,
    pub(super) stats: Option<(u64, u64, u64)>,
    // Latest self audio-control state from OutMsg::State (input mute / output
    // deafen / recording). `None` until the bin reports its first snapshot.
    pub(super) control_state: Option<(bool, bool, bool)>,
}
