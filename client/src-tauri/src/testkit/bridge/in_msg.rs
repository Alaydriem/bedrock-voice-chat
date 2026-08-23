use serde::{Deserialize, Serialize};

// Commands the orchestrator sends to the e2e client over stdin. Each is framed
// as a u32 big-endian length prefix followed by a serde_json body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InMsg {
    InputPcm {
        samples: Vec<f32>,
    },
    TriggerJukebox {
        audio_id: String,
        x: f32,
        y: f32,
        z: f32,
    },
    StartProxy {
        upstream_host: String,
        upstream_port: u16,
        listen_port: u16,
        // Absent resolves the same way a real connect does, which is what every
        // scenario written before this field wanted.
        #[serde(default)]
        addon_mode: Option<common::structs::bedrock::AddonMode>,
    },
    InjectPresence {
        token: String,
    },
    // Request a snapshot of the transport-fidelity counters. The bin responds
    // with OutMsg::Stats immediately.
    RequestStats,
    // Request the live link diagnostics. The bin responds with
    // OutMsg::Diagnostics immediately, or with stalled=false and connected=false
    // when no connection is up.
    RequestDiagnostics,
    // Gracefully tear down the QUIC connection (the production server-switch
    // path) without exiting the process. The bin responds with
    // OutMsg::Disconnected once the network stream has been reset, so the server
    // observes a clean CONNECTION_CLOSE and fires its disconnect cleanup.
    Disconnect,
    // Explicit channel-membership operations over the real HTTP channel-event
    // API, targeting an already-known channel id. These drive the production
    // `api_channel_event` path the UI uses, so the harness exercises the same
    // server-side membership keying.
    LeaveChannel {
        channel_id: String,
    },
    RejoinChannel {
        channel_id: String,
    },
    DeleteChannel {
        channel_id: String,
    },
    // Upload an audio file to the server's library via the real client encode +
    // upload path. `wav_path` is a file the orchestrator wrote; the bin decodes
    // + Opus-muxes + POSTs it. Replies with OutMsg::AudioUploaded.
    UploadAudio {
        wav_path: String,
        game: String,
    },
    Shutdown,
}
