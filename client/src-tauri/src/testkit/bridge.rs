use std::io::{Read, Write};

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
    },
    InjectPresence {
        token: String,
    },
    // Request a snapshot of the transport-fidelity counters. The bin responds
    // with OutMsg::Stats immediately.
    RequestStats,
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

// Upper bound on a single frame so a corrupt length prefix cannot trigger a
// multi-gigabyte allocation. PCM chunks are small; 64 MiB is generous.
const MAX_FRAME_LEN: u32 = 64 * 1024 * 1024;

pub struct Frame;

impl Frame {
    pub fn read<R: Read, T: for<'de> Deserialize<'de>>(reader: &mut R) -> std::io::Result<T> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf);
        if len > MAX_FRAME_LEN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("frame length {len} exceeds maximum {MAX_FRAME_LEN}"),
            ));
        }

        let mut body = vec![0u8; len as usize];
        reader.read_exact(&mut body)?;
        serde_json::from_slice(&body)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn write<W: Write, T: Serialize>(writer: &mut W, value: &T) -> std::io::Result<()> {
        let body = serde_json::to_vec(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let len = u32::try_from(body.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "frame body too large")
        })?;
        writer.write_all(&len.to_be_bytes())?;
        writer.write_all(&body)?;
        writer.flush()
    }
}
