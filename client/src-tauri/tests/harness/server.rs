use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::harness::ffi::{SendHandle, ServerLibrary};

/// A real BVC server booted in-process for integration tests by loading the
/// server's C FFI cdylib (`bvc_server_lib`) with `libloading` — the same
/// embedding path the Java/BDS mod uses. The loaded `ServerLibrary` and the
/// opaque runtime handle are retained so later tasks can drive the server via
/// `bvc_update_positions`, `bvc_audio_play`, and similar FFI calls.
pub struct EmbeddedServer {
    lib: Arc<ServerLibrary>,
    handle: SendHandle,
    rocket_port: u16,
    quic_port: u16,
    certs_path: PathBuf,
    server_thread: Option<JoinHandle<()>>,
}

impl EmbeddedServer {
    /// Bind an ephemeral TCP port, read the assigned port, and release it.
    pub fn free_port_tcp() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .expect("bind ephemeral tcp port")
            .local_addr()
            .expect("read tcp local addr")
            .port()
    }

    /// Bind an ephemeral UDP port, read the assigned port, and release it.
    pub fn free_port_udp() -> u16 {
        std::net::UdpSocket::bind("127.0.0.1:0")
            .expect("bind ephemeral udp port")
            .local_addr()
            .expect("read udp local addr")
            .port()
    }

    /// Build the JSON config string `bvc_server_create` deserializes into
    /// `ApplicationConfig`. Only the fields an in-process test server needs are
    /// set; everything else falls back to the server's serde defaults. The
    /// Rocket serving cert points at the CA the server generates at startup
    /// (`ca.crt`/`ca.key`), which carries ServerAuth EKU and a
    /// `localhost`/`127.0.0.1` SAN, so no separate leaf cert is required.
    pub fn config_json(rocket_port: u16, quic_port: u16, data_dir: &Path) -> String {
        Self::config_json_with_relay(rocket_port, quic_port, data_dir, None)
    }

    /// Binds the wildcard IPv6 address, which serves IPv4 peers as well because
    /// s2n-quic-platform clears IPV6_V6ONLY on the socket it creates. This is the
    /// production default; the other constructors pin `127.0.0.1`, so without this
    /// the dual-stack bind would go entirely untested.
    pub fn config_json_dual_stack(rocket_port: u16, quic_port: u16, data_dir: &Path) -> String {
        Self::config_json_inner(rocket_port, quic_port, data_dir, None, "::", &[])
    }

    /// Advertises a port other than the bound one, which is how a client is steered
    /// through a middlebox: `/api/config` reports the advertised list and the client
    /// dials that, while the server keeps listening on `quic_port`.
    pub fn config_json_advertising(
        rocket_port: u16,
        quic_port: u16,
        data_dir: &Path,
        advertised_quic_ports: &[u16],
    ) -> String {
        Self::config_json_inner(
            rocket_port,
            quic_port,
            data_dir,
            None,
            "127.0.0.1",
            advertised_quic_ports,
        )
    }

    /// Like `config_json` but lowers the cross-server relay cadence so a peer link
    /// converges within a test window. Discovery is decentralized (in-realm `!bvca`
    /// announce); the relay plane builds unconditionally, so the only knobs are the
    /// announce/orchestration cadence (lowered to 1s here from the 60s/5s production
    /// defaults) and `idle_timeout_secs`, which when set lowers the peer-link idle
    /// teardown window so a drop + reconnect is observable (production is 300s).
    pub fn config_json_with_relay(
        rocket_port: u16,
        quic_port: u16,
        data_dir: &Path,
        idle_timeout_secs: Option<u64>,
    ) -> String {
        Self::config_json_inner(
            rocket_port,
            quic_port,
            data_dir,
            idle_timeout_secs,
            "127.0.0.1",
            &[],
        )
    }

    fn config_json_inner(
        rocket_port: u16,
        quic_port: u16,
        data_dir: &Path,
        idle_timeout_secs: Option<u64>,
        listen: &str,
        advertised_quic_ports: &[u16],
    ) -> String {
        let certs_path = data_dir.join("certificates");
        let db_path = data_dir.join("bvc-test.sqlite3");
        let assets_path = data_dir.join("assets");
        let audio_path = data_dir.join("audio");
        std::fs::create_dir_all(&audio_path).expect("create audio storage dir");

        let mut relay = json!({
            "announce_interval_secs": 1,
            "orchestration_interval_secs": 1,
        });
        if let Some(secs) = idle_timeout_secs {
            relay["idle_timeout_secs"] = json!(secs);
        }

        let config = json!({
            "database": {
                "scheme": "sqlite3",
                "database": db_path.to_string_lossy(),
            },
            "log": {
                "level": "warn",
                "out": "stdout",
            },
            "audio": {
                "file_path": audio_path.to_string_lossy(),
            },
            "permissions": {
                "defaults": {
                    "audio_upload": true,
                    "audio_delete": true,
                },
            },
            "server": {
                "listen": listen,
                "port": rocket_port,
                "quic_port": quic_port,
                "advertised_quic_ports": advertised_quic_ports,
                "assets_path": assets_path.to_string_lossy(),
                "tls": {
                    "certificate": certs_path.join("ca.crt").to_string_lossy(),
                    "key": certs_path.join("ca.key").to_string_lossy(),
                    "certs_path": certs_path.to_string_lossy(),
                    "names": ["localhost"],
                    "ips": ["127.0.0.1"],
                },
                "minecraft": {
                    "access_token": "test-token",
                },
                "features": {
                    "code_login": true,
                    "relay": relay,
                },
                "bedrock": {
                    "transfer_port": Self::free_port_udp(),
                },
            },
        });

        config.to_string()
    }

    /// Load the server cdylib from its default location in the server workspace
    /// target dir, caching the handle for the life of the process. The cdylib
    /// must already be built (`cargo build -p bedrock-voice-chat-server` in
    /// `server/`).
    ///
    /// The cache is load-bearing, not an optimization: `bvc_server_destroy`
    /// bounds its tokio shutdown with a timeout, so runtime threads can briefly
    /// outlive destroy. If the last `Arc<ServerLibrary>` dropped during test
    /// teardown, libloading would `FreeLibrary` the DLL and unmap code those
    /// threads are still executing — an intermittent STATUS_ACCESS_VIOLATION
    /// (0xc0000005). Holding the library in a process-lifetime static keeps the
    /// DLL mapped until process exit, which terminates lingering threads before
    /// the loader unmaps anything — the same load-once-never-unload contract
    /// production embedders (JVM/BDS) follow.
    pub fn load_library() -> Arc<ServerLibrary> {
        static LIBRARY: OnceLock<Arc<ServerLibrary>> = OnceLock::new();
        LIBRARY
            .get_or_init(|| {
                Self::constrain_runtime();
                let path = ServerLibrary::default_lib_path();
                ServerLibrary::load(&path).expect("load server cdylib")
            })
            .clone()
    }

    /// Cap each embedded server's tokio pools for the test process. Production /
    /// the Java mod leave these unset and keep tokio's CPU-scaled defaults; the
    /// suite embeds several servers per process and runs many processes in
    /// parallel, so without this the aggregate thread/handle count faults the
    /// native crypto/QUIC deps. Read by `bvc_server_create` in the cdylib.
    fn constrain_runtime() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            // SAFETY: set exactly once, before any server runtime is created and
            // before any thread reads these vars, from the harness entry point.
            unsafe {
                std::env::set_var("BVC_RUNTIME_WORKER_THREADS", "2");
                std::env::set_var("BVC_RUNTIME_MAX_BLOCKING_THREADS", "16");
            }
        });
    }

    /// Boot the server: create the runtime from the JSON config, spawn the
    /// blocking `bvc_server_start` on a dedicated thread, and poll
    /// `/api/config` until it returns 200 or the boot timeout elapses.
    pub async fn start(
        lib: Arc<ServerLibrary>,
        config_json: &str,
        rocket_port: u16,
        quic_port: u16,
        certs_path: &Path,
    ) -> Self {
        let c_config = CString::new(config_json).expect("config json contains no nul byte");

        let handle = unsafe { (lib.create)(c_config.as_ptr()) };
        if handle.is_null() {
            let err = Self::last_error(&lib);
            panic!("bvc_server_create returned null: {err}");
        }
        let handle = SendHandle(handle);

        let start_lib = lib.clone();
        let start_handle = handle;
        let server_thread = std::thread::Builder::new()
            .name(format!("bvc-embedded-server-{rocket_port}"))
            .spawn(move || {
                let handle = start_handle;
                let rc = unsafe { (start_lib.start)(handle.0) };
                if rc != 0 {
                    eprintln!(
                        "bvc_server_start returned {rc}: {}",
                        Self::last_error(&start_lib)
                    );
                }
            })
            .expect("spawn embedded server thread");

        let server = Self {
            lib,
            handle,
            rocket_port,
            quic_port,
            certs_path: certs_path.to_path_buf(),
            server_thread: Some(server_thread),
        };

        server.await_ready().await;
        server
    }

    pub fn rocket_port(&self) -> u16 {
        self.rocket_port
    }

    pub fn quic_port(&self) -> u16 {
        self.quic_port
    }

    /// The loaded library, for tasks that drive the server via further FFI calls.
    pub fn library(&self) -> Arc<ServerLibrary> {
        self.lib.clone()
    }

    /// The opaque runtime handle, for further FFI calls against this server.
    pub fn handle(&self) -> SendHandle {
        self.handle
    }

    /// Provision a Minecraft player (idempotent create) and mint a fresh
    /// single-use login code via `bvc_provision_login_code`, so a client can
    /// later redeem it through the real `code_login` flow. The returned
    /// CString is copied into an owned `String` and freed via `bvc_free_string`.
    pub fn login_code(&self, gamertag: &str) -> String {
        let c_gamertag = CString::new(gamertag).expect("gamertag contains no nul byte");
        let c_game = CString::new("minecraft").expect("game contains no nul byte");

        let ptr = unsafe {
            (self.lib.provision_login_code)(
                self.handle.0,
                c_gamertag.as_ptr(),
                c_game.as_ptr(),
                300,
            )
        };

        if ptr.is_null() {
            let err = Self::last_error(&self.lib);
            panic!("bvc_provision_login_code returned null: {err}");
        }

        let code = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();

        unsafe { (self.lib.free_string)(ptr) };

        code
    }

    /// Mint a single-use WebSocket ticket for a gamertag.
    ///
    /// Provisioned rather than fetched: the HTTP route trades an mTLS identity for a ticket
    /// and answers ncryptf-encrypted, so reaching it from here would mean reimplementing
    /// both to watch a feed. Same seam `login_code` uses, and the ticket itself is no
    /// different from an HTTP-issued one.
    pub fn websocket_ticket(&self, gamertag: &str) -> String {
        let c_gamertag = CString::new(gamertag).expect("gamertag contains no nul byte");
        let c_game = CString::new("minecraft").expect("game contains no nul byte");

        let ptr = unsafe {
            (self.lib.provision_websocket_ticket)(
                self.handle.0,
                c_gamertag.as_ptr(),
                c_game.as_ptr(),
            )
        };

        if ptr.is_null() {
            let err = Self::last_error(&self.lib);
            panic!("bvc_provision_websocket_ticket returned null: {err}");
        }

        let ticket = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();

        unsafe { (self.lib.free_string)(ptr) };

        ticket
    }

    /// Read snapshots from `/api/websocket/positions` for one observer.
    ///
    /// Returns once `wanted` snapshots carrying at least one entry have arrived, or the
    /// deadline passes — whichever comes first. Empty frames are counted but not returned:
    /// an observer the world does not know about yet produces them normally, and a test
    /// waiting on positions wants the ones that say something.
    pub async fn position_snapshots(
        &self,
        gamertag: &str,
        wanted: usize,
        timeout: Duration,
    ) -> Vec<common::structs::position::PositionSnapshot> {
        use futures_util::StreamExt;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        use tokio_tungstenite::tungstenite::http::HeaderValue;

        let ticket = self.websocket_ticket(gamertag);
        let url = format!("wss://127.0.0.1:{}/api/websocket/positions", self.rocket_port);

        let mut request = url.into_client_request().expect("build ws request");
        // The credential travels as a subprotocol rather than a header or a query parameter,
        // because a browser can offer subprotocols and cannot set headers — and a ticket in
        // a URL lands in every access log between here and the server.
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str(&format!("ticket.{ticket}, bvc.positions.v1"))
                .expect("ticket is header-safe"),
        );

        let connector = tokio_tungstenite::Connector::Rustls(Arc::new(
            crate::harness::insecure_tls::trust_anything(),
        ));
        let (mut socket, _) = tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(connector),
        )
        .await
        .expect("connect position feed");

        let mut out = Vec::new();
        let deadline = Instant::now() + timeout;
        while out.len() < wanted && Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Ok(Some(Ok(message))) = tokio::time::timeout(remaining, socket.next()).await else {
                break;
            };
            if let tokio_tungstenite::tungstenite::Message::Text(body) = message {
                if let Ok(snapshot) =
                    serde_json::from_str::<common::structs::position::PositionSnapshot>(&body)
                {
                    if !snapshot.positions.is_empty() {
                        out.push(snapshot);
                    }
                }
            }
        }
        out
    }

    /// Drive player positions into the server's QUIC fan-out via the
    /// `bvc_update_positions` FFI.  Each entry is `(name, x, y, z)`.  The game
    /// is always `"minecraft"` and non-position fields are defaulted (overworld
    /// dimension, no deafen/spectator, zero orientation).
    ///
    /// The `name` is the bare gamertag (e.g. `"Alice"`, not `"minecraft:Alice"`), because
    /// this is the payload a game mod sends. The server composes the canonical identity from
    /// it and the game before caching, so a bare name here is correct and a prefixed one
    /// would produce `minecraft:minecraft:Alice`.
    pub fn update_positions(&self, players: &[(&str, f32, f32, f32)]) {
        let player_jsons: Vec<serde_json::Value> = players
            .iter()
            .map(|(name, x, y, z)| {
                json!({
                    "name": name,
                    "coordinates": { "x": x, "y": y, "z": z },
                    "orientation": { "x": 0.0, "y": 0.0 },
                    "dimension": "overworld",
                    "deafen": false,
                    "spectator": false
                })
            })
            .collect();

        let payload = json!({
            "game": "minecraft",
            "players": player_jsons,
        });

        let json_str = payload.to_string();
        let c_json = CString::new(json_str).expect("position json contains no nul byte");

        let rc = unsafe { (self.lib.update_positions)(self.handle.0, c_json.as_ptr()) };
        if rc != 0 {
            let err = Self::last_error(&self.lib);
            panic!("bvc_update_positions returned {rc}: {err}");
        }
    }

    /// The generated root CA in PEM form, for clients that pin the server root.
    pub fn ca_pem(&self) -> String {
        let path = self.certs_path.join("ca.crt");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(pem) = std::fs::read_to_string(&path) {
                return pem;
            }
            if Instant::now() >= deadline {
                panic!("ca.crt never appeared at {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Trigger HTTP-driven jukebox playback at a world position, exactly as the
    /// BDS mod does (`POST /api/audio/event` with X-MC-Access-Token). The JSON
    /// shape matches the mod's AudioPlayRequest: `game` is the serde(tag="game")
    /// GameAudioContext enum, nested. Returns (event_id, duration_ms).
    pub async fn jukebox_play(&self, audio_file_id: &str, x: f32, y: f32, z: f32) -> (String, u32) {
        let url = format!("https://127.0.0.1:{}/api/audio/event", self.rocket_port);
        let body = serde_json::json!({
            "audio_file_id": audio_file_id,
            "game": {
                "game": "minecraft",
                "coordinates": { "x": x, "y": y, "z": z },
                "dimension": "overworld",
                "world_uuid": "00000000-0000-0000-0000-000000000000",
            },
        });
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build jukebox client");
        let resp = client
            .post(&url)
            .header("X-MC-Access-Token", "test-token")
            .json(&body)
            .send()
            .await
            .expect("POST /api/audio/event");
        assert!(
            resp.status().is_success(),
            "jukebox play failed: {}",
            resp.status()
        );
        let v: serde_json::Value = resp.json().await.expect("parse AudioEventResponse");
        let event_id = v["event_id"].as_str().expect("event_id").to_string();
        let duration_ms = v["duration_ms"].as_u64().unwrap_or(0) as u32;
        (event_id, duration_ms)
    }

    /// Eject/stop an active jukebox playback (`DELETE /api/audio/event/{id}`).
    pub async fn jukebox_stop(&self, event_id: &str) {
        let url = format!(
            "https://127.0.0.1:{}/api/audio/event/{}",
            self.rocket_port, event_id
        );
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build jukebox client");
        let resp = client
            .delete(&url)
            .header("X-MC-Access-Token", "test-token")
            .send()
            .await
            .expect("DELETE /api/audio/event");
        assert!(
            resp.status().is_success(),
            "jukebox stop failed: {}",
            resp.status()
        );
    }

    /// Submit a self-action over the real HTTP control plane (`POST /api/control`),
    /// exactly as a mod does — actor attributed by `gamertag`, gated by the MC token.
    /// Drives `SetMuted` so the harness can assert the actor's client applied it.
    pub async fn post_control_setmuted(&self, gamertag: &str, muted: bool) {
        let url = format!("https://127.0.0.1:{}/api/control", self.rocket_port);
        let body = serde_json::json!({
            "id": gamertag,
            "action": { "SetMuted": muted },
        });
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build control client");
        let resp = client
            .post(&url)
            .header("X-MC-Access-Token", "test-token")
            .json(&body)
            .send()
            .await
            .expect("POST /api/control");
        assert!(
            resp.status().is_success(),
            "control setmuted failed: {}",
            resp.status()
        );
    }

    /// Submit a per-player volume preference over the control plane
    /// (`POST /api/control` with a `SetVolume` action), actor attributed by `gamertag`.
    pub async fn post_control_setvolume(&self, gamertag: &str, target: &str, volume: f32) {
        let url = format!("https://127.0.0.1:{}/api/control", self.rocket_port);
        let body = serde_json::json!({
            "id": gamertag,
            "action": { "SetVolume": { "target": target, "volume": volume } },
        });
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build control client");
        let resp = client
            .post(&url)
            .header("X-MC-Access-Token", "test-token")
            .json(&body)
            .send()
            .await
            .expect("POST /api/control");
        assert!(
            resp.status().is_success(),
            "control setvolume failed: {}",
            resp.status()
        );
    }

    /// Read a player's cached self-state (`GET /api/state?id=`). `None` until the
    /// client's QueryState reporter has seeded the cache.
    pub async fn get_state(&self, gamertag: &str) -> Option<serde_json::Value> {
        let url = format!(
            "https://127.0.0.1:{}/api/state?id={}",
            self.rocket_port, gamertag
        );
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build state client");
        let resp = client
            .get(&url)
            .header("X-MC-Access-Token", "test-token")
            .send()
            .await
            .expect("GET /api/state");
        assert!(
            resp.status().is_success(),
            "get state failed: {}",
            resp.status()
        );
        let text = resp.text().await.unwrap_or_default();
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(serde_json::Value::Null) | Err(_) => None,
            Ok(v) => Some(v),
        }
    }

    /// Poll `/api/state` until `pred` passes or `timeout` elapses; returns the
    /// matching state, or `None` on timeout.
    pub async fn await_state<F>(
        &self,
        gamertag: &str,
        pred: F,
        timeout: std::time::Duration,
    ) -> Option<serde_json::Value>
    where
        F: Fn(&serde_json::Value) -> bool,
    {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(state) = self.get_state(gamertag).await {
                if pred(&state) {
                    return Some(state);
                }
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
    }

    /// Read the owner's cached per-player preferences scoped to `targets`
    /// (`GET /api/preferences?owner=&targets=`, comma-separated targets).
    pub async fn get_preferences(&self, owner: &str, targets: &str) -> Vec<serde_json::Value> {
        let url = format!(
            "https://127.0.0.1:{}/api/preferences?owner={}&targets={}",
            self.rocket_port, owner, targets
        );
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build preferences client");
        let resp = client
            .get(&url)
            .header("X-MC-Access-Token", "test-token")
            .send()
            .await
            .expect("GET /api/preferences");
        assert!(
            resp.status().is_success(),
            "get preferences failed: {}",
            resp.status()
        );
        resp.json::<Vec<serde_json::Value>>().await.unwrap_or_default()
    }

    /// Poll `/api/preferences` until the owner's preference for `target` passes
    /// `pred` or `timeout` elapses; returns the matching preference.
    pub async fn await_preference<F>(
        &self,
        owner: &str,
        target: &str,
        pred: F,
        timeout: std::time::Duration,
    ) -> Option<serde_json::Value>
    where
        F: Fn(&serde_json::Value) -> bool,
    {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(pref) = self
                .get_preferences(owner, target)
                .await
                .into_iter()
                .find(|p| p["target"] == serde_json::Value::String(target.to_string()))
            {
                if pred(&pref) {
                    return Some(pref);
                }
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
    }

    /// Read the library's thread-local last-error string.
    fn last_error(lib: &ServerLibrary) -> String {
        let ptr = unsafe { (lib.last_error)() };
        if ptr.is_null() {
            return "<no error>".to_string();
        }
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    /// Poll `GET https://127.0.0.1:{rocket_port}/api/config` until it returns a
    /// success status or the boot timeout elapses.
    async fn await_ready(&self) {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build polling reqwest client");
        let url = format!("https://127.0.0.1:{}/api/config", self.rocket_port);
        let deadline = Instant::now() + Duration::from_secs(30);

        loop {
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    return;
                }
            }
            if Instant::now() >= deadline {
                panic!(
                    "embedded server did not serve {} within the boot timeout",
                    url
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl Drop for EmbeddedServer {
    /// Signal shutdown, join the server thread, then destroy the handle. Order
    /// matters: `bvc_server_destroy` must not run while `bvc_server_start` is
    /// still on the thread, so the join comes first.
    fn drop(&mut self) {
        unsafe { (self.lib.stop)(self.handle.0) };

        if let Some(thread) = self.server_thread.take() {
            let _ = thread.join();
        }

        unsafe { (self.lib.destroy)(self.handle.0) };
    }
}
