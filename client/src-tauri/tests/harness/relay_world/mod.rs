#![allow(dead_code)]

mod actor_spec;
mod relay_player;
mod srv;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use bedrock_client::BedrockClient;
use bvc_client_lib::testkit::signal::Signal;
use common::bedrock_protocol::AuthInfo;
use common::bedrock_protocol::version::ProtocolVersion;
use tempfile::TempDir;

use crate::harness::client_proc::ClientProc;
use crate::harness::proxy_driver::FakeBedrockUpstream;
use crate::harness::proxy_scale::Scale;
use crate::harness::server::EmbeddedServer;

pub use actor_spec::ActorSpec;
pub use srv::Srv;

use relay_player::RelayPlayer;

/// A cross-server relay test world: two federating voice servers A and B (and an
/// optional C), and one fake realm upstream per distinct realm. Discovery is
/// decentralized — each server announces its endpoint into the realm (`!bvca`),
/// which the fake upstream rebroadcasts, so there is no central discovery relay.
/// Each actor is a real `bvc_client_e2e` proc that voice-connects to its assigned
/// server and proxies its assigned realm. Because A and B share no voice server
/// and the actors join no channel, the relay peer link is the only path audio can
/// cross.
pub struct RelayWorld {
    server_a: EmbeddedServer,
    server_b: EmbeddedServer,
    // Booted only when an actor targets `Srv::C` — the 2-server scenarios pay no
    // third-runtime cost.
    server_c: Option<EmbeddedServer>,
    upstreams: Vec<FakeBedrockUpstream>,
    players: HashMap<String, RelayPlayer>,
    realm_of: HashMap<String, usize>,
    version: ProtocolVersion,
    _data_dirs: Vec<TempDir>,
}

impl RelayWorld {
    /// Boot the relay, servers A/B, one upstream per `realms` entry (its world
    /// name), and every actor in `actors`.
    pub async fn boot(version: ProtocolVersion, realms: &[&str], actors: &[ActorSpec<'_>]) -> Self {
        Self::boot_with_idle(version, realms, actors, None).await
    }

    /// Like `boot` but lowers the federating servers' peer-link idle teardown
    /// window to `idle_timeout_secs` (production is 300s), so a drop + reconnect is
    /// observable within a test window. `None` keeps the production default.
    pub async fn boot_with_idle(
        version: ProtocolVersion,
        realms: &[&str],
        actors: &[ActorSpec<'_>],
        idle_timeout_secs: Option<u64>,
    ) -> Self {
        let lib = EmbeddedServer::load_library();

        let d_a = tempfile::tempdir().expect("server A data dir");
        let d_b = tempfile::tempdir().expect("server B data dir");

        let (ra, qa) = (
            EmbeddedServer::free_port_tcp(),
            EmbeddedServer::free_port_udp(),
        );
        let (rb, qb) = (
            EmbeddedServer::free_port_tcp(),
            EmbeddedServer::free_port_udp(),
        );

        let cfg_a = EmbeddedServer::config_json_with_relay(ra, qa, d_a.path(), idle_timeout_secs);
        let cfg_b = EmbeddedServer::config_json_with_relay(rb, qb, d_b.path(), idle_timeout_secs);

        let server_a = EmbeddedServer::start(
            lib.clone(),
            &cfg_a,
            ra,
            qa,
            &d_a.path().join("certificates"),
        )
        .await;
        let server_b = EmbeddedServer::start(
            lib.clone(),
            &cfg_b,
            rb,
            qb,
            &d_b.path().join("certificates"),
        )
        .await;

        // A third federating server is only needed by scenarios that place an
        // actor on `Srv::C`; boot it on demand so the common 2-server cases keep
        // their three-handle footprint.
        let needs_c = actors.iter().any(|a| matches!(a.server, Srv::C));
        let (server_c, d_c) = if needs_c {
            let d_c = tempfile::tempdir().expect("server C data dir");
            let (rc, qc) = (
                EmbeddedServer::free_port_tcp(),
                EmbeddedServer::free_port_udp(),
            );
            let cfg_c =
                EmbeddedServer::config_json_with_relay(rc, qc, d_c.path(), idle_timeout_secs);
            let server_c = EmbeddedServer::start(
                lib.clone(),
                &cfg_c,
                rc,
                qc,
                &d_c.path().join("certificates"),
            )
            .await;
            (Some(server_c), Some(d_c))
        } else {
            (None, None)
        };

        let url_a = format!("https://127.0.0.1:{}", server_a.rocket_port());
        let url_b = format!("https://127.0.0.1:{}", server_b.rocket_port());
        let url_c = server_c
            .as_ref()
            .map(|s| format!("https://127.0.0.1:{}", s.rocket_port()));

        let mut upstreams = Vec::new();
        for name in realms {
            upstreams.push(FakeBedrockUpstream::bind_named(version, name).await);
        }

        let mut players = HashMap::new();
        let mut realm_of = HashMap::new();

        for actor in actors {
            let url = match actor.server {
                Srv::A => url_a.as_str(),
                Srv::B => url_b.as_str(),
                Srv::C => url_c.as_deref().expect("server C booted for Srv::C actor"),
            };
            let code = match actor.server {
                Srv::A => server_a.login_code(actor.name),
                Srv::B => server_b.login_code(actor.name),
                Srv::C => server_c
                    .as_ref()
                    .expect("server C booted for Srv::C actor")
                    .login_code(actor.name),
            };
            let proc = ClientProc::spawn(actor.name, &code, url, "");
            proc.await_connected(Duration::from_secs(30))
                .expect("voice connect");

            let listen = EmbeddedServer::free_port_udp();
            let upstream = &mut upstreams[actor.realm];
            let upstream_addr = upstream.addr();
            proc.start_proxy(
                &upstream_addr.ip().to_string(),
                upstream_addr.port(),
                listen,
                None,
                Duration::from_secs(10),
            )
            .expect("proxy started");

            let proxy_addr: SocketAddr = format!("127.0.0.1:{listen}").parse().expect("proxy addr");
            let connect =
                BedrockClient::connect(proxy_addr, AuthInfo::offline(actor.name), version);
            let accept = upstream.accept_player();
            let (downstream_res, accepted) = tokio::join!(connect, accept);
            let downstream = downstream_res.expect("downstream connects to proxy");
            assert_eq!(
                accepted, actor.name,
                "upstream connection identity must match actor"
            );

            upstream.start_game(actor.name).await;

            players.insert(
                actor.name.to_string(),
                RelayPlayer {
                    proc,
                    _downstream: downstream,
                },
            );
            realm_of.insert(actor.name.to_string(), actor.realm);
        }

        let mut data_dirs = vec![d_a, d_b];
        if let Some(d_c) = d_c {
            data_dirs.push(d_c);
        }

        Self {
            server_a,
            server_b,
            server_c,
            upstreams,
            players,
            realm_of,
            version,
            _data_dirs: data_dirs,
        }
    }

    pub fn proc(&self, name: &str) -> &ClientProc {
        &self.players.get(name).expect("known player").proc
    }

    pub fn server_a(&self) -> &EmbeddedServer {
        &self.server_a
    }

    pub fn server_b(&self) -> &EmbeddedServer {
        &self.server_b
    }

    pub fn server_c(&self) -> &EmbeddedServer {
        self.server_c.as_ref().expect("server C booted")
    }

    /// Drive `name`'s position via PlayerAuthInput on the upstream of its realm.
    pub async fn drive_position(&mut self, name: &str, x: f32, y: f32, z: f32) {
        let realm = self.realm_of[name];
        self.upstreams[realm].drive_position(name, x, y, z).await;
    }

    /// Trigger a jukebox insert (bvc:play) through `name`'s proxy.
    pub async fn play_sound(
        &mut self,
        name: &str,
        audio_id: &str,
        x: i32,
        y: i32,
        z: i32,
        dim: &str,
    ) {
        let realm = self.realm_of[name];
        self.upstreams[realm]
            .play_sound(name, audio_id, x, y, z, dim)
            .await;
    }

    /// Trigger a jukebox eject (bvc:eject) through `name`'s proxy.
    pub async fn eject(&mut self, name: &str, x: i32, y: i32, z: i32, dim: &str) {
        let realm = self.realm_of[name];
        self.upstreams[realm].eject(name, x, y, z, dim).await;
    }

    /// One convergence/refresh pump: drive every position, fan out any pending
    /// `!bvcp` presence chat on every realm, then sleep `gap`. Run on a loop to
    /// converge the peer link (the relay proof rides the realm fan-out) and to
    /// keep positions fresh against the proxy's ~250ms heartbeat.
    pub async fn pump(&mut self, positions: &[(&str, f32, f32, f32)], rounds: u32, gap: Duration) {
        for _ in 0..rounds {
            for (name, x, y, z) in positions {
                let realm = self.realm_of[*name];
                self.upstreams[realm].drive_position(name, *x, *y, *z).await;
            }
            for up in &mut self.upstreams {
                up.rebroadcast_presence_chat().await;
            }
            tokio::time::sleep(gap).await;
        }
    }

    /// Drive positions + fan presence chat while feeding `speaker` the `warmup`
    /// tone until `listener` receives QUIC frames (the cross-server peer link is
    /// up and relaying) or `budget` elapses. Returns the listener's cumulative
    /// `frames_from_quic` at exit — callers gate on it as the relay-established
    /// guard before any "hears across servers" assertion. The warmup frames are
    /// counted before the measured phase, so callers snapshot counters AFTER this
    /// returns to measure a clean delivery delta.
    pub async fn converge_link(
        &mut self,
        speaker: &str,
        listener: &str,
        positions: &[(&str, f32, f32, f32)],
        warmup: &[f32],
        budget: Duration,
    ) -> u64 {
        let deadline = std::time::Instant::now() + budget;
        loop {
            self.proc(speaker).feed_tone(warmup, 48_000);
            self.pump(positions, 8, Duration::from_millis(180)).await;
            let fq = self.proc(listener).stats().1;
            if fq > 0 || std::time::Instant::now() >= deadline {
                return fq;
            }
        }
    }

    /// Drive positions/presence while every speaker feeds its probe tone, until every
    /// `(listener, speaker_scale)` pair is audible inside a single probe window, or
    /// `budget` elapses. Returns the pairs still inaudible at exit; empty means the
    /// mesh currently carries every direction the caller is about to measure.
    ///
    /// Gating on `frames_from_quic` instead cannot express this. That counter is an
    /// aggregate, so a listener sharing a server with one speaker, or holding one of
    /// three mesh edges, satisfies it while the direction under test still carries
    /// nothing. Each mesh direction depends on its own server having learned the far
    /// player's presence and position, and those propagate independently — so "some
    /// frames arrived" is true well before "this speaker reaches this listener" is.
    /// Measuring from that point reads `Scale::hears` over a window whose early part
    /// predates delivery, and the energy fraction lands under the threshold even
    /// though the edge did come up.
    pub async fn converge_audible(
        &mut self,
        speakers: &[(&str, &[f32])],
        pairs: &[(&str, Scale)],
        positions: &[(&str, f32, f32, f32)],
        budget: Duration,
    ) -> Vec<String> {
        // One window must outlast the probe tone, or a pair is judged on a partial
        // delivery and the loop spins without ever reading a converged mesh.
        const PROBE_ROUNDS: u32 = 16;
        const PROBE_GAP: Duration = Duration::from_millis(180);

        let deadline = std::time::Instant::now() + budget;
        loop {
            for (listener, _) in pairs {
                let _ = self.proc(listener).drain_captured();
            }
            for (name, tone) in speakers {
                self.proc(name).feed_tone(tone, 48_000);
            }
            self.pump(positions, PROBE_ROUNDS, PROBE_GAP).await;

            let mut captures: HashMap<&str, Vec<f32>> = HashMap::new();
            for (listener, _) in pairs {
                captures
                    .entry(listener)
                    .or_insert_with(|| Signal::to_mono(&self.proc(listener).drain_captured()));
            }

            let missing: Vec<String> = pairs
                .iter()
                .filter(|(listener, scale)| !Scale::hears(&captures[listener], *scale))
                .map(|(listener, scale)| {
                    format!(
                        "{listener}<-{} {}",
                        scale.name,
                        Scale::why(&captures[listener], *scale)
                    )
                })
                .collect();

            if missing.is_empty() || std::time::Instant::now() >= deadline {
                return missing;
            }
        }
    }

    pub fn shutdown(self) {
        for (_n, p) in self.players {
            p.proc.shutdown();
        }
    }
}
