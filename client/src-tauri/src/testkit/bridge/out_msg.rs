use serde::{Deserialize, Serialize};

// Events the e2e client emits to the orchestrator over stdout, framed
// identically to `InMsg`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutMsg {
    Ready,
    Connected,
    // Emitted once the QUIC connection has been gracefully torn down in response
    // to InMsg::Disconnect.
    Disconnected,
    // Emitted after the connect sequence joins a channel, carrying the
    // server-assigned channel id so the orchestrator can later target it with
    // explicit leave/rejoin/delete operations.
    ChannelJoined {
        channel_id: String,
    },
    // Emitted after an explicit channel-membership operation
    // (LeaveChannel/RejoinChannel/DeleteChannel) completes, echoing the op name.
    ChannelOpDone {
        op: String,
    },
    // Emitted after UploadAudio: the server-assigned audio file id + its decoded
    // duration, so the orchestrator can trigger jukebox playback of it.
    AudioUploaded {
        audio_file_id: String,
        duration_ms: u32,
    },
    CapturedPcm {
        samples: Vec<f32>,
    },
    Log {
        line: String,
    },
    // Emitted after StartProxy completes, carrying the local port the proxy is
    // listening on so the orchestrator can connect a downstream client to it.
    ProxyStarted {
        listen_port: u16,
    },
    // Transport-fidelity counter snapshot emitted in response to InMsg::RequestStats.
    // frames_sent      — Opus AudioFrame packets this client emitted to the QUIC bus.
    // frames_from_quic — AudioFrame packets this client received from the QUIC bus
    //                    (post-network, before local routing in handle_audio_data).
    // frames_into_jitter_buffer
    //                  — EncodedAudioFramePacket objects forwarded into the jitter
    //                    buffer pipeline (handle_audio_data succeeded). This is
    //                    ingest into the pipeline, BEFORE playback drain — not a
    //                    "heard" count.
    // Link diagnostics emitted in response to InMsg::RequestDiagnostics. Only the fields a
    // scenario asserts on are surfaced; the full snapshot is not worth serialising through the
    // bridge.
    Diagnostics {
        connected: bool,
        stalled: bool,
        uptime_secs: u64,
        datagrams_sent: u64,
        datagrams_received: u64,
        // Speakers currently attributable in the per-peer table, which is what the support log
        // line is built from.
        peers: Vec<String>,
        // Server-to-client loss as the client derived it from the server's per-connection sequence.
        // `None` means unmeasured, which is a different claim from zero and must survive the bridge
        // as such.
        downlink_loss_pct: Option<f32>,
        // Which transport actually carried this session, as the client reports it. A
        // scenario that only asserts audio arrived cannot tell QUIC from WebSocket, and a
        // fallback test that silently ran on QUIC would pass while proving nothing.
        transport: Option<String>,
    },
    Stats {
        frames_sent: u64,
        frames_from_quic: u64,
        frames_into_jitter_buffer: u64,
    },
    // The client's self audio-control state (input mute / output deafen / recording).
    // Emitted whenever it changes so the orchestrator can assert control effects
    // (e.g. a ClientBound ClientAction muting the actor).
    State {
        muted: bool,
        deafened: bool,
        recording: bool,
    },
    // The backend announced a player_gain_store change — the same event the
    // dashboard's player cards re-render on. Carries the persisted store as
    // JSON at that moment, so the orchestrator can assert both that the event
    // fired and what state a card would render.
    GainStoreUpdated {
        store_json: String,
    },
    // A frontend-facing Tauri event observed at the webview boundary, forwarded
    // verbatim (name + raw JSON payload). These are the render triggers the
    // desktop UI consumes; scenarios assert them so a broken contract (event
    // not fired, renamed, or malformed payload) fails e2e instead of waiting
    // for manual QA.
    UiEvent {
        event: String,
        payload: String,
    },
}
