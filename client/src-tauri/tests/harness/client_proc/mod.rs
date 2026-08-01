use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bvc_client_lib::testkit::bridge::{Frame, InMsg, OutMsg};

mod shared_state;

use shared_state::SharedState;

/// Drives the `bvc_client_e2e` bin as a child process over the framed
/// stdin/stdout protocol from `bvc_client_lib::testkit::bridge`. Sends `InMsg`
/// commands down stdin and folds the bin's `OutMsg` events into shared state on
/// a background reader thread.
///
/// The e2e harness must be PRE-BUILT before tests spawn a `ClientProc`:
/// `cargo build -p bvc-client-e2e`
/// (it lands in the ROOT workspace target dir, `target/debug/`, since the harness
/// crate belongs to the root workspace).
pub struct ClientProc {
    child: Child,
    stdin: ChildStdin,
    state: Arc<Mutex<SharedState>>,
    reader: Option<JoinHandle<()>>,
}

// App-data namespace the e2e bin writes to (its `generate_context` identifier is
// overridden to this so nothing lands in the real client's dir). Wiped once per
// test-binary run so the isolated store/cookies/WebView2 data never accumulate.
const E2E_APP_DATA_IDENTIFIER: &str = "com.alaydriem.bvc.client.e2e";

impl ClientProc {
    fn wipe_e2e_app_data_once() {
        static WIPE: std::sync::Once = std::sync::Once::new();
        WIPE.call_once(|| {
            for var in ["APPDATA", "LOCALAPPDATA"] {
                if let Ok(base) = std::env::var(var) {
                    let dir = std::path::Path::new(&base).join(E2E_APP_DATA_IDENTIFIER);
                    let _ = std::fs::remove_dir_all(&dir);
                }
            }
        });
    }
}

impl ClientProc {
    /// Platform-specific bin file name produced by the e2e bin target.
    fn bin_file_name() -> &'static str {
        if cfg!(windows) {
            "bvc_client_e2e.exe"
        } else {
            "bvc_client_e2e"
        }
    }

    /// Resolve the e2e bin path. The client crate lives in the ROOT workspace
    /// (`client/src-tauri`), so its artifacts land in the root `target/debug`,
    /// two levels up from this crate's manifest dir.
    pub fn bin_path() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir)
            .join("..")
            .join("..")
            .join("target")
            .join("debug")
            .join(Self::bin_file_name())
    }

    /// Spawn the e2e bin with the `BVC_E2E_*` env populated and stdin/stdout
    /// piped. A background thread immediately starts draining framed `OutMsg`
    /// from stdout into shared state.
    pub fn spawn(gamertag: &str, login_code: &str, server_url: &str, channel: &str) -> ClientProc {
        Self::spawn_with_channel_id(gamertag, login_code, server_url, channel, None)
    }

    /// Like `spawn` but accepts a pre-existing channel id. When `channel_id` is
    /// `Some`, the bin skips channel creation and joins the supplied id directly.
    /// Use this when two clients must share an already-created channel.
    pub fn spawn_with_channel_id(
        gamertag: &str,
        login_code: &str,
        server_url: &str,
        channel: &str,
        channel_id: Option<&str>,
    ) -> ClientProc {
        let bin = Self::bin_path();
        assert!(
            bin.exists(),
            "e2e harness not found at {}; build it first with \
             `cargo build -p bvc-client-e2e`",
            bin.display()
        );

        // The e2e bin scopes all its app-data (store.json, the audio input path's
        // own store read, webview store/cookies) under the `.e2e` identifier; clear
        // any leftovers from a prior run so they never accumulate.
        Self::wipe_e2e_app_data_once();

        let mut cmd = Command::new(&bin);
        cmd.env("BVC_E2E_SERVER", server_url)
            .env("BVC_E2E_GAMERTAG", gamertag)
            .env("BVC_E2E_CODE", login_code)
            .env("BVC_E2E_CHANNEL", channel);
        if let Some(id) = channel_id {
            cmd.env("BVC_E2E_CHANNEL_ID", id);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn e2e bin {}: {e}", bin.display()));

        let stdin = child.stdin.take().expect("child stdin piped");
        let mut stdout = child.stdout.take().expect("child stdout piped");

        let state = Arc::new(Mutex::new(SharedState::default()));
        let reader_state = state.clone();
        let reader = std::thread::Builder::new()
            .name("client-proc-stdout".to_string())
            .spawn(move || {
                loop {
                    match Frame::read::<_, OutMsg>(&mut stdout) {
                        Ok(OutMsg::Ready) => reader_state.lock().unwrap().ready = true,
                        Ok(OutMsg::Connected) => reader_state.lock().unwrap().connected = true,
                        Ok(OutMsg::Disconnected) => {
                            reader_state.lock().unwrap().disconnected = true
                        }
                        Ok(OutMsg::ChannelJoined { channel_id }) => {
                            reader_state.lock().unwrap().channel_id = Some(channel_id);
                        }
                        Ok(OutMsg::ChannelOpDone { op }) => {
                            reader_state.lock().unwrap().last_channel_op = Some(op);
                        }
                        Ok(OutMsg::AudioUploaded {
                            audio_file_id,
                            duration_ms,
                        }) => {
                            reader_state.lock().unwrap().last_upload =
                                Some((audio_file_id, duration_ms));
                        }
                        Ok(OutMsg::ProxyStarted { listen_port }) => {
                            reader_state.lock().unwrap().proxy_listen = Some(listen_port);
                        }
                        Ok(OutMsg::CapturedPcm { samples }) => {
                            reader_state
                                .lock()
                                .unwrap()
                                .captured
                                .extend_from_slice(&samples);
                        }
                        Ok(OutMsg::Log { line }) => {
                            eprintln!("[bvc_client_e2e] {line}");
                        }
                        Ok(OutMsg::Diagnostics {
                            connected,
                            stalled,
                            uptime_secs,
                            peers,
                            downlink_loss_pct,
                            ..
                        }) => {
                            let mut guard = reader_state.lock().unwrap();
                            guard.diagnostics = Some((connected, stalled, uptime_secs));
                            guard.diagnostic_peers = peers;
                            guard.diagnostic_downlink_loss = Some(downlink_loss_pct);
                        }
                        Ok(OutMsg::Stats {
                            frames_sent,
                            frames_from_quic,
                            frames_into_jitter_buffer,
                        }) => {
                            reader_state.lock().unwrap().stats =
                                Some((frames_sent, frames_from_quic, frames_into_jitter_buffer));
                        }
                        Ok(OutMsg::UiEvent { event, payload }) => {
                            reader_state.lock().unwrap().ui_events.push((event, payload));
                        }
                        Ok(OutMsg::GainStoreUpdated { store_json }) => {
                            if let Ok(v) = serde_json::from_str(&store_json) {
                                reader_state.lock().unwrap().gain_store = Some(v);
                            }
                        }
                        Ok(OutMsg::State {
                            muted,
                            deafened,
                            recording,
                        }) => {
                            reader_state.lock().unwrap().control_state =
                                Some((muted, deafened, recording));
                        }
                        Err(_) => break,
                    }
                }
            })
            .expect("spawn client-proc stdout reader");

        ClientProc {
            child,
            stdin,
            state,
            reader: Some(reader),
        }
    }

    /// Block until the bin has emitted `Ready` or `timeout` elapses.
    pub fn await_ready(&self, timeout: Duration) -> Result<(), String> {
        self.await_flag(timeout, |s| s.ready, "Ready")
    }

    /// Block until the bin has emitted `Connected` or `timeout` elapses.
    pub fn await_connected(&self, timeout: Duration) -> Result<(), String> {
        self.await_flag(timeout, |s| s.connected, "Connected")
    }

    /// Block until the bin reports its input-mute state equals `want` (via
    /// `OutMsg::State`), or `timeout` elapses.
    pub fn await_muted(&self, want: bool, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some((muted, _, _)) = self.state.lock().unwrap().control_state {
                if muted == want {
                    return Ok(());
                }
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "timed out waiting for muted=={want} after {timeout:?}"
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Block until the persisted `player_gain_store` snapshot carried by the
    /// card-render event (`OutMsg::GainStoreUpdated`) satisfies `pred`, or
    /// `timeout` elapses. Never satisfied if the event never fires — this is
    /// the render-trigger assertion, not a store poll.
    pub fn await_gain_store<F>(&self, pred: F, timeout: Duration) -> Result<serde_json::Value, String>
    where
        F: Fn(&serde_json::Value) -> bool,
    {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(store) = self.state.lock().unwrap().gain_store.clone() {
                if pred(&store) {
                    return Ok(store);
                }
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "timed out waiting for a matching gain-store event after {timeout:?}"
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Block until the bin has observed frontend-facing Tauri event `event`
    /// with a payload satisfying `pred`, or `timeout` elapses. Returns the
    /// matching payload. This asserts the RENDER TRIGGER the webview consumes,
    /// at the boundary — not the DOM it produces.
    pub fn await_ui_event<F>(
        &self,
        event: &str,
        pred: F,
        timeout: Duration,
    ) -> Result<String, String>
    where
        F: Fn(&str) -> bool,
    {
        let deadline = Instant::now() + timeout;
        loop {
            {
                let state = self.state.lock().unwrap();
                if let Some((_, payload)) = state
                    .ui_events
                    .iter()
                    .find(|(name, payload)| name == event && pred(payload))
                {
                    return Ok(payload.clone());
                }
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "timed out waiting for ui event {event} after {timeout:?}"
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Gracefully tear down the QUIC connection (without exiting the process)
    /// and block until the bin confirms with `Disconnected`. This drives the
    /// production server-switch path so the server sees a clean CONNECTION_CLOSE
    /// and runs its disconnect cleanup promptly.
    pub fn disconnect(&self, timeout: Duration) -> Result<(), String> {
        self.send(&InMsg::Disconnect);
        self.await_flag(timeout, |s| s.disconnected, "Disconnected")
    }

    /// Block until the bin reports the channel id it joined during connect, or
    /// `timeout` elapses.
    pub fn await_channel_id(&self, timeout: Duration) -> Result<String, String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(id) = self.state.lock().unwrap().channel_id.clone() {
                return Ok(id);
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "timed out waiting for ChannelJoined after {timeout:?}"
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Explicitly leave the given channel over the real HTTP channel-event API
    /// and block until the bin acknowledges.
    pub fn leave_channel(&self, channel_id: &str, timeout: Duration) -> Result<(), String> {
        self.channel_op(
            InMsg::LeaveChannel {
                channel_id: channel_id.to_string(),
            },
            "leave",
            timeout,
        )
    }

    /// Re-join the given channel over the real HTTP channel-event API and block
    /// until the bin acknowledges.
    pub fn rejoin_channel(&self, channel_id: &str, timeout: Duration) -> Result<(), String> {
        self.channel_op(
            InMsg::RejoinChannel {
                channel_id: channel_id.to_string(),
            },
            "rejoin",
            timeout,
        )
    }

    /// Disband (delete) the given channel over the real HTTP channel-event API
    /// and block until the bin acknowledges.
    pub fn delete_channel(&self, channel_id: &str, timeout: Duration) -> Result<(), String> {
        self.channel_op(
            InMsg::DeleteChannel {
                channel_id: channel_id.to_string(),
            },
            "delete",
            timeout,
        )
    }

    /// Upload a WAV through the client's real encode+upload path; block until the
    /// bin reports the server-assigned id. Returns (audio_file_id, duration_ms).
    pub fn upload_audio(
        &self,
        wav_path: &str,
        game: &str,
        timeout: Duration,
    ) -> Result<(String, u32), String> {
        self.state.lock().unwrap().last_upload = None;
        self.send(&InMsg::UploadAudio {
            wav_path: wav_path.to_string(),
            game: game.to_string(),
        });
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(u) = self.state.lock().unwrap().last_upload.clone() {
                return Ok(u);
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "timed out waiting for AudioUploaded after {timeout:?}"
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Tell the bin to start its Bedrock proxy pointed at `upstream_host:upstream_port`
    /// and listening on `listen_port`, then block until the bin confirms with
    /// `ProxyStarted` or `timeout` elapses. Returns `Ok(())` once the proxy is up.
    pub fn start_proxy(
        &self,
        upstream_host: &str,
        upstream_port: u16,
        listen_port: u16,
        timeout: Duration,
    ) -> Result<(), String> {
        self.state.lock().unwrap().proxy_listen = None;
        self.send(&InMsg::StartProxy {
            upstream_host: upstream_host.to_string(),
            upstream_port,
            listen_port,
        });
        let deadline = Instant::now() + timeout;
        loop {
            if self.state.lock().unwrap().proxy_listen == Some(listen_port) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("timed out waiting for ProxyStarted".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Send one channel-membership command and block until the bin echoes a
    /// matching `ChannelOpDone`. The completion latch is cleared first so a
    /// prior op's ack is never mistaken for this one.
    fn channel_op(&self, msg: InMsg, op: &str, timeout: Duration) -> Result<(), String> {
        self.state.lock().unwrap().last_channel_op = None;
        self.send(&msg);

        let deadline = Instant::now() + timeout;
        loop {
            if self.state.lock().unwrap().last_channel_op.as_deref() == Some(op) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "timed out waiting for channel op '{op}' after {timeout:?}"
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Poll the shared state until `pred` holds or the deadline passes.
    fn await_flag(
        &self,
        timeout: Duration,
        pred: impl Fn(&SharedState) -> bool,
        label: &str,
    ) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        loop {
            if pred(&self.state.lock().unwrap()) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!("timed out waiting for {label} after {timeout:?}"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Deliver the full probe `pcm` to the bin promptly as ~20 ms `InputPcm`
    /// frames with NO wall-clock sleep between sends. The orchestrator no longer
    /// paces delivery: the fake input source on the bin side clocks PCM out to
    /// the DSP at an accurate 20 ms cadence (like a real microphone), so the
    /// whole clip is enqueued in milliseconds here and the bin's FrameClock
    /// governs real-time pacing into the QUIC send path.
    pub fn feed_tone(&self, pcm: &[f32], sample_rate: u32) {
        let frame = (sample_rate / 50).max(1) as usize;
        for chunk in pcm.chunks(frame) {
            self.send(&InMsg::InputPcm {
                samples: chunk.to_vec(),
            });
        }
    }

    /// Request a transport-counter snapshot from the bin and block until it
    /// arrives (or the 5 s deadline passes). Returns `(frames_sent,
    /// frames_from_quic, frames_into_jitter_buffer)` as recorded by the atomic
    /// counters in the real audio path.
    pub fn stats(&self) -> (u64, u64, u64) {
        // Clear any stale snapshot before requesting a fresh one.
        self.state.lock().unwrap().stats = None;
        self.send(&InMsg::RequestStats);

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(s) = self.state.lock().unwrap().stats {
                return s;
            }
            if Instant::now() >= deadline {
                return (0, 0, 0);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Request a link-diagnostics reading from the bin and block until it arrives (or the 5 s
    /// deadline passes). Returns `(connected, stalled, uptime_secs)`.
    ///
    /// The bin reads the real service, so this observes the same derivation a player's status
    /// panel and copyable report would show.
    pub fn diagnostics(&self) -> (bool, bool, u64) {
        {
            let mut guard = self.state.lock().unwrap();
            guard.diagnostics = None;
            guard.diagnostic_downlink_loss = None;
        }
        self.send(&InMsg::RequestDiagnostics);

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(d) = self.state.lock().unwrap().diagnostics {
                return d;
            }
            if Instant::now() >= deadline {
                return (false, false, 0);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Derived downlink loss as of the last `diagnostics()` call.
    ///
    /// The outer `Option` distinguishes "no reading arrived" from the inner one, which is the client
    /// reporting the figure unmeasured because no stamped envelope has been seen.
    pub fn diagnostic_downlink_loss(&self) -> Option<Option<f32>> {
        self.state.lock().unwrap().diagnostic_downlink_loss
    }

    /// Speaker names in the per-peer diagnostics table as of the last `diagnostics()` call.
    pub fn diagnostic_peers(&self) -> Vec<String> {
        self.state.lock().unwrap().diagnostic_peers.clone()
    }

    /// Polls until the derived downlink loss satisfies `pred`, returning the value that matched.
    ///
    /// Loss is a windowed figure recomputed on the service's own tick, so a scenario has to sample
    /// repeatedly rather than read once — an immediate read after arming loss would observe the
    /// window before it.
    pub fn await_diagnostics_downlink_loss<F>(
        &self,
        pred: F,
        timeout: Duration,
    ) -> Result<f32, String>
    where
        F: Fn(Option<f32>) -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut last: Option<Option<f32>> = None;
        while Instant::now() < deadline {
            let _ = self.diagnostics();
            let reading = self.diagnostic_downlink_loss().flatten();
            last = Some(self.diagnostic_downlink_loss().flatten());
            if pred(reading) {
                return reading.ok_or_else(|| "predicate matched an unmeasured reading".to_string());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Err(format!(
            "downlink loss predicate never held; last reading {last:?}"
        ))
    }

    /// Polls diagnostics until `pred` holds or the deadline passes. The service derives a stall
    /// across consecutive samples, so a scenario has to sample repeatedly rather than once.
    pub fn await_diagnostics<F>(&self, pred: F, timeout: Duration) -> Result<(bool, bool, u64), String>
    where
        F: Fn(&(bool, bool, u64)) -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut last = (false, false, 0);
        while Instant::now() < deadline {
            last = self.diagnostics();
            if pred(&last) {
                return Ok(last);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Err(format!("diagnostics predicate never held; last reading {last:?}"))
    }

    /// Drain captured samples accumulated over `dur`. Sleeps the full window so
    /// the reader thread has time to fold in late-arriving `CapturedPcm`, then
    /// snapshots and clears the buffer.
    pub fn collect_captured(&self, dur: Duration) -> Vec<f32> {
        std::thread::sleep(dur);
        let mut state = self.state.lock().unwrap();
        std::mem::take(&mut state.captured)
    }

    /// Snapshot and clear the accumulated captured samples without sleeping.
    /// Use when the caller controls the sleep externally (e.g. concurrent
    /// multi-player capture where a single shared sleep window covers all procs).
    pub fn drain_captured(&self) -> Vec<f32> {
        let mut state = self.state.lock().unwrap();
        std::mem::take(&mut state.captured)
    }

    /// Signal the bin to exit, then wait for the process and reader thread.
    pub fn shutdown(mut self) {
        self.send(&InMsg::Shutdown);
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }

    /// Write one framed `InMsg` to the child's stdin. Frame errors are swallowed
    /// so a dead child doesn't panic the orchestrator mid-test.
    fn send(&self, msg: &InMsg) {
        let mut stdin = &self.stdin;
        if Frame::write(&mut stdin, msg).is_ok() {
            let _ = stdin.flush();
        }
    }
}

impl Drop for ClientProc {
    /// Best-effort cleanup if `shutdown` was never called: kill the child and
    /// join the reader so no orphaned process or thread survives the test.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
