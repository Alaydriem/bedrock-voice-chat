//! Voice over the WebSocket transport, and voice across both transports at once.
//!
//! These are the tests that decide whether the swarm run is worth doing. Blocking the QUIC
//! port there proves nothing unless the fallback is already known to carry audio; if it is
//! broken, the swarm reports silence and gives no clue which of a dozen moving parts
//! caused it.
//!
//! Requires both artifacts pre-built:
//! * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
//! * e2e harness: `cargo build -p bvc-client-e2e`

use std::time::{Duration, Instant};

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;
use crate::harness::udp_blackhole::UdpBlackhole;

// Covers the ~3 s paced send (≈150 frames × 20 ms) plus transport, jitter and decode
// drain. The WebSocket jitter floor is 140 ms against QUIC's 60 ms, so the extra headroom
// over the send duration matters more here than on the QUIC twin.
const COLLECT_WINDOW: Duration = Duration::from_millis(4_000);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);

/// A ceiling on how long a player on a blocked network waits before they can speak.
///
/// The reachability probe must spend a full handshake budget to establish that nothing
/// answers on UDP — that cost is irreducible, because proving silence takes as long as
/// waiting for it. What this guards is spending it *twice*: an earlier revision walked the
/// same dead endpoints again after the probe had already condemned them, and the player
/// paid both budgets back to back. Anything above this means that regression is back.
const MAX_FALLBACK_CONNECT: Duration = Duration::from_secs(12);

struct Heard {
    rms: f32,
    c4: f32,
    e4: f32,
    g4: f32,
}

impl Heard {
    fn from(captured: &[f32]) -> Self {
        let mono = Signal::to_mono(captured);
        Self {
            rms: Signal::rms(&mono),
            c4: Signal::tone_energy_fraction(&mono, 48_000, 261.63),
            e4: Signal::tone_energy_fraction(&mono, 48_000, 329.63),
            g4: Signal::tone_energy_fraction(&mono, 48_000, 392.00),
        }
    }

    /// The same gate the QUIC audio scenarios use: audible signal, and each note of the
    /// probe chord carrying more than 2 % of the captured energy.
    fn assert_heard(&self, who: &str) {
        assert!(
            self.rms > 0.005,
            "FAIL — {who} is silent (rms={:.6}): the transport carried no audio",
            self.rms
        );
        assert!(
            self.c4 > 0.02 && self.e4 > 0.02 && self.g4 > 0.02,
            "FAIL — {who} did not hear the probe chord \
             (c4={:.4} e4={:.4} g4={:.4}, each needs > 2 %)",
            self.c4,
            self.e4,
            self.g4
        );
    }
}

/// Asserts a client is running on the transport the scenario says it is.
///
/// The load-bearing check in this file. Every other assertion here — frames delivered,
/// chord energy — passes identically whichever transport carried the audio, so without
/// this a fallback test that quietly ran on QUIC reads as a green fallback test.
fn assert_transport(client: &ClientProc, who: &str, expected: &str) {
    let reported = client.transport();
    assert_eq!(
        reported.as_deref(),
        Some(expected),
        "FAIL — {who} was expected on the {expected} transport but reports {reported:?}"
    );
}

/// A client whose every advertised QUIC port is blackholed still gets a voice session.
///
/// ## What this proves
///
/// The whole point of the transport. The server advertises only a port that swallows
/// datagrams, so the connect walk cannot complete a handshake; the client records the
/// verdict, dials `wss://` on the API's own port, and the demultiplexer routes it to the
/// voice listener by ALPN. Audio then flows.
///
/// The blackhole's counter is asserted non-zero: without it a passing test could mean the
/// client never tried QUIC at all, which would prove nothing about the fallback.
#[tokio::test(flavor = "multi_thread")]
async fn a_blocked_quic_port_falls_back_to_the_websocket_transport() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let blackhole = UdpBlackhole::start().await;

    // The only QUIC endpoint this client is ever told about swallows datagrams — a blocked
    // UDP path, not a closed one, so the client learns nothing until its budget expires.
    let config_json =
        EmbeddedServer::config_json_quic_unreachable(rocket_port, data_dir.path(), blackhole.port());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server = EmbeddedServer::start(lib, &config_json, rocket_port, 0, &certs_path).await;

    let alice_code = server.login_code("Alice");
    let bob_code = server.login_code("Bob");
    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let started = Instant::now();
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "blocked-quic");
    alice
        .await_connected(CONNECT_TIMEOUT)
        .expect("Alice connects over WebSocket after QUIC is blocked");
    let fallback_took = started.elapsed();

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "blocked-quic");
    bob.await_connected(CONNECT_TIMEOUT)
        .expect("Bob connects over WebSocket after QUIC is blocked");

    // Before anything else: neither client may have quietly reached QUIC. Every advertised
    // port is blackholed, so a QUIC session here means a port leaked into the candidate set
    // and this scenario is measuring the wrong transport.
    assert_transport(&alice, "Alice", "websocket");
    assert_transport(&bob, "Bob", "websocket");

    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 1.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(600));

    let probe = Signal::musical_probe(48_000);
    let alice_probe = probe.clone();
    let feed_handle = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe, 48_000);
        alice
    });

    let captured = bob.collect_captured(COLLECT_WINDOW);
    let alice = feed_handle.join().expect("feed thread panicked");

    let (alice_sent, _, _) = alice.stats();
    let (_, bob_received, _) = bob.stats();
    let swallowed = blackhole.swallowed();

    alice.shutdown();
    bob.shutdown();

    let heard = Heard::from(&captured);
    eprintln!(
        "[ws_transport/blocked] fallback_took={fallback_took:?} swallowed={swallowed} \
         alice_sent={alice_sent} \
         bob_received={bob_received} rms={:.6} c4={:.4} e4={:.4} g4={:.4}",
        heard.rms, heard.c4, heard.e4, heard.g4
    );

    assert!(
        fallback_took < MAX_FALLBACK_CONNECT,
        "FAIL — a player on a blocked network waited {fallback_took:?} to connect, over \
         the {MAX_FALLBACK_CONNECT:?} ceiling. The probe already proved UDP is silent; \
         something is spending that budget a second time"
    );
    assert!(
        swallowed > 0,
        "FAIL — nothing reached the blackhole, so the client never attempted QUIC; \
         this run proves nothing about the fallback"
    );
    assert!(
        alice_sent > 0,
        "FAIL — Alice sent zero AudioFrames: the input pipeline never produced frames"
    );

    // TCP does not lose frames. Where the QUIC twin asserts equality and tolerates the
    // occasional contended shortfall, this one has no such excuse available.
    assert_eq!(
        bob_received, alice_sent,
        "FAIL — WebSocket delivery loss: Alice sent {alice_sent} AudioFrames, Bob received \
         {bob_received}. A reliable, ordered transport must not drop any"
    );

    heard.assert_heard("Bob");
}

/// Channel members on the WebSocket transport hear each other at any distance.
///
/// The twin of `same_channel::same_channel_members_hear_regardless_of_distance`. Same
/// 10 000 block separation, same channel-membership bypass, different transport — so a
/// routing decision that silently depended on the QUIC path would fail here and pass
/// there.
#[tokio::test(flavor = "multi_thread")]
async fn websocket_channel_members_hear_regardless_of_distance() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let alice_code = server.login_code("Alice");
    let bob_code = server.login_code("Bob");
    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let alice = ClientProc::spawn_websocket("Alice", &alice_code, &url, "ws-shared-voice");
    alice
        .await_connected(CONNECT_TIMEOUT)
        .expect("Alice connects over WebSocket + joins ws-shared-voice");

    let bob = ClientProc::spawn_websocket("Bob", &bob_code, &url, "ws-shared-voice");
    bob.await_connected(CONNECT_TIMEOUT)
        .expect("Bob connects over WebSocket + joins ws-shared-voice");

    assert_transport(&alice, "Alice", "websocket");
    assert_transport(&bob, "Bob", "websocket");

    // Far beyond every proximity gate; only channel membership can carry this.
    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(600));

    let probe = Signal::musical_probe(48_000);
    let alice_probe = probe.clone();
    let feed_handle = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe, 48_000);
        alice
    });

    let captured = bob.collect_captured(COLLECT_WINDOW);
    let alice = feed_handle.join().expect("feed thread panicked");

    let (alice_sent, _, _) = alice.stats();
    let (_, bob_received, _) = bob.stats();

    alice.shutdown();
    bob.shutdown();

    let heard = Heard::from(&captured);
    eprintln!(
        "[ws_transport/channel] alice_sent={alice_sent} bob_received={bob_received} \
         rms={:.6} c4={:.4} e4={:.4} g4={:.4}",
        heard.rms, heard.c4, heard.e4, heard.g4
    );

    assert!(alice_sent > 0, "FAIL — Alice sent zero AudioFrames");
    assert_eq!(
        bob_received, alice_sent,
        "FAIL — WebSocket delivery loss in the same channel: Alice sent {alice_sent}, \
         Bob received {bob_received}"
    );
    heard.assert_heard("Bob");
}

/// A WebSocket client and a QUIC client in one channel hear each other, both ways.
///
/// ## What this proves
///
/// The premise the whole design rests on: one session implementation behind two
/// transports. Both clients register in the same `ConnectionRegistry`, are routed by the
/// same code, and are stamped with device ids from disjoint spaces. If any of that is
/// wrong — a registry key that collides, a routing arm that reaches for a QUIC connection,
/// a stamp that only the QUIC path applies — this is the test that fails, and every
/// single-transport test still passes.
///
/// Transport is chosen per client process, because the server's advertised configuration
/// is shared by everyone connected to it and cannot express a split.
#[tokio::test(flavor = "multi_thread")]
async fn a_websocket_client_and_a_quic_client_hear_each_other() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let alice_code = server.login_code("Alice");
    let bob_code = server.login_code("Bob");
    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    // Alice on WebSocket, Bob on QUIC, one channel.
    let alice = ClientProc::spawn_websocket("Alice", &alice_code, &url, "mixed-voice");
    alice
        .await_connected(CONNECT_TIMEOUT)
        .expect("Alice connects over WebSocket + creates mixed-voice");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "mixed-voice");
    bob.await_connected(CONNECT_TIMEOUT)
        .expect("Bob connects over QUIC + joins mixed-voice");

    // Without this the scenario is two clients on an unknown pair of transports, which is
    // exactly what it must not be: the premise under test is that ONE session
    // implementation serves both, and that is only exercised if they genuinely differ.
    assert_transport(&alice, "Alice", "websocket");
    assert_transport(&bob, "Bob", "quic");

    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(600));

    let probe = Signal::musical_probe(48_000);

    // WebSocket -> QUIC.
    let alice_probe = probe.clone();
    let feed_handle = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe, 48_000);
        alice
    });
    let bob_captured = bob.collect_captured(COLLECT_WINDOW);
    let alice = feed_handle.join().expect("feed thread panicked");
    let (alice_sent, _, _) = alice.stats();

    // QUIC -> WebSocket, on the same pair, so a one-directional break cannot pass.
    let bob_probe = probe.clone();
    let bob_feed = std::thread::spawn(move || {
        bob.feed_tone(&bob_probe, 48_000);
        bob
    });
    let alice_captured = alice.collect_captured(COLLECT_WINDOW);
    let bob = bob_feed.join().expect("feed thread panicked");
    let (bob_sent, _, _) = bob.stats();

    alice.shutdown();
    bob.shutdown();

    let bob_heard = Heard::from(&bob_captured);
    let alice_heard = Heard::from(&alice_captured);

    eprintln!(
        "[ws_transport/mixed] ws->quic alice_sent={alice_sent} \
         bob rms={:.6} c4={:.4} e4={:.4} g4={:.4} | \
         quic->ws bob_sent={bob_sent} \
         alice rms={:.6} c4={:.4} e4={:.4} g4={:.4}",
        bob_heard.rms,
        bob_heard.c4,
        bob_heard.e4,
        bob_heard.g4,
        alice_heard.rms,
        alice_heard.c4,
        alice_heard.e4,
        alice_heard.g4,
    );

    assert!(
        alice_sent > 0,
        "FAIL — the WebSocket client sent zero AudioFrames"
    );
    assert!(bob_sent > 0, "FAIL — the QUIC client sent zero AudioFrames");

    bob_heard.assert_heard("Bob (QUIC), listening to a WebSocket speaker");
    alice_heard.assert_heard("Alice (WebSocket), listening to a QUIC speaker");
}
