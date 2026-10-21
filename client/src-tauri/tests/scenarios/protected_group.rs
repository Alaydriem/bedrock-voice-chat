//! End-to-end encrypted groups, through real clients and a real server.
//!
//! Each client is the e2e bin, so every step runs the production path: the group service, the
//! key socket, admission between members, MLS with the X-Wing suite, SFrame sealing in the capture
//! pipeline and opening in the router. The server routes ciphertext it cannot read.
//!
//! Requires the server cdylib and the e2e bin (`mise run test-client` builds both).

use std::time::Duration;

use bvc_client_lib::testkit::WireCapture;
use bvc_client_lib::testkit::signal::Signal;
use common::structs::packet::{AudioFramePacket, QuicNetworkPacket};

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

const PASSWORD: &str = "correct horse";
const CONNECT: Duration = Duration::from_secs(30);
const ADMIT: Duration = Duration::from_secs(20);

struct World {
    server: EmbeddedServer,
    url: String,
    _data: tempfile::TempDir,
}

impl World {
    async fn start() -> Self {
        let data = tempfile::tempdir().expect("create temp data dir");
        let rocket_port = EmbeddedServer::free_port_tcp();
        let quic_port = EmbeddedServer::free_port_udp();
        let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data.path());
        let certs_path = data.path().join("certificates");
        let lib = EmbeddedServer::load_library();
        let server =
            EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;
        let url = format!("https://127.0.0.1:{}", server.rocket_port());
        Self {
            server,
            url,
            _data: data,
        }
    }

    /// A connected client in no channel.
    fn client(&self, name: &str) -> ClientProc {
        let code = self.server.login_code(name);
        let client = ClientProc::spawn(name, &code, &self.url, "");
        client
            .await_connected(CONNECT)
            .unwrap_or_else(|e| panic!("{name} connects: {e}"));
        client
    }

    fn place(&self, positions: &[(&str, f32, f32, f32)]) {
        for _ in 0..5 {
            self.server.update_positions(positions);
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(600));
    }
}

fn joined(client: &ClientProc, name: &str) {
    client
        .await_ui_event("protected-group:joined", |_| true, ADMIT)
        .unwrap_or_else(|e| panic!("{name} is admitted: {e}"));
}

/// Every byte a client handed the server: what the server could observe, whatever it does with it.
fn sent(capture: &WireCapture) -> impl Iterator<Item = &Vec<u8>> {
    capture
        .datagrams
        .iter()
        .chain(&capture.key_frames)
        .chain(&capture.http_bodies)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|window| window == needle)
}

fn audio_frames(capture: &WireCapture) -> Vec<AudioFramePacket> {
    capture
        .datagrams
        .iter()
        .filter_map(|bytes| QuicNetworkPacket::from_datagram(bytes).ok())
        .filter_map(|packet| AudioFramePacket::try_from(packet.data).ok())
        .collect()
}

fn epoch_of(payload: &str) -> u64 {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|v| v["epoch"].as_u64())
        .unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread")]
async fn protected_group_three_clients_hear_each_other() {
    let world = World::start().await;
    let alice = world.client("Alice");
    let bob = world.client("Bob");
    let carol = world.client("Carol");

    let group = alice.create_protected(PASSWORD, ADMIT).expect("Alice creates the group");
    joined(&alice, "Alice");
    bob.join_protected(&group, PASSWORD, ADMIT).expect("Bob starts joining");
    joined(&bob, "Bob");
    carol.join_protected(&group, PASSWORD, ADMIT).expect("Carol starts joining");
    joined(&carol, "Carol");

    // Far apart: only group routing can carry this audio.
    world.place(&[
        ("Alice", 0.0, 64.0, 0.0),
        ("Bob", 10_000.0, 64.0, 0.0),
        ("Carol", -10_000.0, 64.0, 0.0),
    ]);

    let probe = Signal::musical_probe(48_000);
    let alice_probe = probe.clone();
    let feed = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe, 48_000);
        alice
    });
    let captured = bob.collect_captured(Duration::from_millis(3_600));
    let alice = feed.join().expect("feed thread");

    let (alice_sent, _, _) = alice.stats();
    let (_, _, bob_heard) = bob.await_transport_frames(alice_sent, Duration::from_secs(5));
    let (_, _, carol_heard) = carol.await_transport_frames(alice_sent, Duration::from_secs(5));
    let rms = Signal::rms(&Signal::to_mono(&captured));
    eprintln!(
        "[protected_group] alice_sent={alice_sent} bob_heard={bob_heard} carol_heard={carol_heard} rms={rms:.6}"
    );

    alice.shutdown();
    bob.shutdown();
    carol.shutdown();

    assert!(alice_sent > 0, "Alice sealed and sent no frames");
    assert!(bob_heard > 0, "Bob opened none of Alice's sealed frames");
    assert!(carol_heard > 0, "Carol opened none of Alice's sealed frames");
    assert!(rms > 0.005, "Bob's decrypted audio is silent (rms {rms:.6})");
}

#[tokio::test(flavor = "multi_thread")]
async fn protected_group_wrong_password_is_refused() {
    let world = World::start().await;
    let alice = world.client("Alice");
    let dave = world.client("Dave");

    let group = alice.create_protected(PASSWORD, ADMIT).expect("Alice creates the group");
    joined(&alice, "Alice");
    dave.join_protected(&group, "not the password", ADMIT)
        .expect("Dave starts joining");

    let lost = dave
        .await_ui_event("protected-group:lost", |_| true, ADMIT)
        .expect("Dave is refused");

    alice.shutdown();
    dave.shutdown();

    assert!(lost.contains("wrong_password"), "lost for the wrong reason: {lost}");
}

#[tokio::test(flavor = "multi_thread")]
async fn protected_member_leaving_advances_the_epoch() {
    let world = World::start().await;
    let alice = world.client("Alice");
    let bob = world.client("Bob");
    let carol = world.client("Carol");

    let group = alice.create_protected(PASSWORD, ADMIT).expect("Alice creates the group");
    joined(&alice, "Alice");
    bob.join_protected(&group, PASSWORD, ADMIT).expect("Bob starts joining");
    joined(&bob, "Bob");
    carol.join_protected(&group, PASSWORD, ADMIT).expect("Carol starts joining");
    joined(&carol, "Carol");
    let with_carol = alice
        .await_ui_event("protected-group:epoch", |p| epoch_of(p) >= 2, ADMIT)
        .expect("Alice reaches the epoch that added Carol");

    carol.shutdown();

    let alice_after = alice
        .await_ui_event("protected-group:epoch", |p| epoch_of(p) > epoch_of(&with_carol), ADMIT)
        .expect("Alice's epoch advances when Carol leaves");
    let bob_after = bob
        .await_ui_event("protected-group:epoch", |p| epoch_of(p) == epoch_of(&alice_after), ADMIT)
        .expect("Bob reaches the same epoch");
    let roster = alice
        .await_ui_event("protected-group:roster", |p| !p.contains("Carol") && p.contains("Bob"), ADMIT)
        .expect("Carol leaves Alice's roster");

    alice.shutdown();
    bob.shutdown();

    let code = |payload: &str| {
        serde_json::from_str::<serde_json::Value>(payload).unwrap()["safety_code"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    };
    assert_eq!(code(&alice_after), code(&bob_after), "Alice and Bob disagree on the group state");
    assert!(!roster.contains("Carol"));
}

// A protected member is not heard in-world: a player standing beside Alice, outside the group,
// receives nothing from her while Bob, in the group, does.
#[tokio::test(flavor = "multi_thread")]
async fn protected_member_is_not_heard_by_nearby_non_member() {
    let world = World::start().await;
    let alice = world.client("Alice");
    let bob = world.client("Bob");
    let erin = world.client("Erin");

    let group = alice.create_protected(PASSWORD, ADMIT).expect("Alice creates the group");
    joined(&alice, "Alice");
    bob.join_protected(&group, PASSWORD, ADMIT).expect("Bob starts joining");
    joined(&bob, "Bob");

    world.place(&[
        ("Alice", 0.0, 64.0, 0.0),
        ("Erin", 2.0, 64.0, 0.0),
        ("Bob", 10_000.0, 64.0, 0.0),
    ]);

    let probe = Signal::musical_probe(48_000);
    alice.feed_tone(&probe, 48_000);
    std::thread::sleep(Duration::from_millis(3_600));

    let (alice_sent, _, _) = alice.stats();
    let (_, _, bob_heard) = bob.await_transport_frames(alice_sent, Duration::from_secs(5));
    let (_, erin_from_quic, erin_heard) = erin.stats();
    eprintln!(
        "[protected_group] alice_sent={alice_sent} bob_heard={bob_heard} erin_from_quic={erin_from_quic} erin_heard={erin_heard}"
    );

    alice.shutdown();
    bob.shutdown();
    erin.shutdown();

    assert!(bob_heard > 0, "Alice was not sending: Bob heard nothing either");
    assert_eq!(erin_from_quic, 0, "a nearby non-member received Alice's frames");
}

// What the server observes, checked against every byte both clients handed it: audio leaves only
// as sealed frames that carry no position, none of the plaintext Opus appears anywhere in what was
// sent, the password crosses no boundary (the PAKE included), and frame sizes follow the frame
// counter rather than the speech.
#[tokio::test(flavor = "multi_thread")]
async fn server_observes_only_ciphertext_of_speech_independent_size_and_never_the_password() {
    const CANARY: &str = "canary-Zq7!vault";
    // Long enough that a match is no coincidence, short enough to catch any fragment of a frame.
    const FRAGMENT: usize = 16;

    let world = World::start().await;
    let alice = world.client("Alice");
    let bob = world.client("Bob");

    let group = alice.create_protected(CANARY, ADMIT).expect("Alice creates the group");
    joined(&alice, "Alice");
    bob.join_protected(&group, CANARY, ADMIT).expect("Bob starts joining");
    joined(&bob, "Bob");
    world.place(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);
    let admission = [alice.wire_capture(), bob.wire_capture()];

    let probe = Signal::musical_probe(48_000);
    alice.feed_tone(&probe, 48_000);
    // The probe plays out at real time after `feed_tone` returns.
    std::thread::sleep(Duration::from_millis(3_600));
    let (alice_sent, _, _) = alice.stats();
    let (_, _, bob_heard) = bob.await_transport_frames(alice_sent, Duration::from_secs(5));
    let speech = alice.wire_capture();

    alice.shutdown();
    bob.shutdown();

    let frames = audio_frames(&speech);
    let sealed_sizes: Vec<usize> = frames.iter().map(|frame| frame.data.len()).collect();
    let plaintext_sizes: Vec<usize> = speech.sealed_plaintext.iter().map(Vec::len).collect();
    eprintln!(
        "[protected_group] frames={} plaintext={} bob_heard={bob_heard} sealed_sizes={:?} plaintext_sizes={:?}",
        frames.len(),
        speech.sealed_plaintext.len(),
        sealed_sizes.iter().collect::<std::collections::BTreeSet<_>>(),
        plaintext_sizes.iter().collect::<std::collections::BTreeSet<_>>(),
    );

    assert!(bob_heard > 0, "the session carried no audio, so nothing below was exercised");
    // Well over a hundred frames, so the size check spans enough of the probe's changing notes for
    // a variable-bitrate encoder to have shown itself.
    assert!(frames.len() >= 100, "too few audio frames to judge: {}", frames.len());
    assert!(!speech.sealed_plaintext.is_empty(), "Alice sealed nothing while speaking");

    for frame in &frames {
        assert!(frame.is_sealed(), "an audio frame left Alice unsealed");
        assert!(frame.speaker.is_none(), "a sealed frame carried Alice's position");
        assert!(frame.spatial.is_none(), "a sealed frame carried a spatial flag");
    }

    let everything_sent: Vec<&Vec<u8>> = admission.iter().chain([&speech]).flat_map(sent).collect();
    for plaintext in &speech.sealed_plaintext {
        let fragment = &plaintext[..FRAGMENT.min(plaintext.len())];
        assert!(
            !everything_sent.iter().any(|bytes| contains(bytes, fragment)),
            "plaintext Opus reached the server"
        );
    }
    assert!(
        !everything_sent.iter().any(|bytes| contains(bytes, CANARY.as_bytes())),
        "the group password reached the server"
    );

    let plaintext_size = plaintext_sizes[0];
    assert!(
        plaintext_sizes.iter().all(|size| *size == plaintext_size),
        "the encoder was not constant-bitrate while protected: {plaintext_sizes:?}"
    );
    assert!(
        sealed_sizes.windows(2).all(|pair| pair[0] <= pair[1]),
        "sealed frame sizes moved with the audio rather than the frame counter: {sealed_sizes:?}"
    );
    assert!(
        sealed_sizes
            .iter()
            .all(|size| (plaintext_size + 17..=plaintext_size + 24).contains(size)),
        "sealed frames carried more than the SFrame overhead: {sealed_sizes:?}"
    );
}
