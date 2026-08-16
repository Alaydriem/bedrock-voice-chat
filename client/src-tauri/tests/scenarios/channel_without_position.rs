use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// Channel members hear each other with NO position data at all.
///
/// ## What this proves
///
/// Joining a voice group is an identity relationship, not a spatial one, so it must
/// work before either player has joined the game. This is the "joined the group in
/// BVC, not yet on the Minecraft server" case.
///
/// The client never populates `AudioFramePacket.sender` itself — it always sends
/// `None` and the server fills it in from `player_cache` during
/// `update_coordinates`. So a player who has sent no position leaves the sender
/// `None` for the whole route. Routing used to read the sender's and recipient's
/// *game* out of that `PlayerEnum` in order to build the `game:gamertag` channel key,
/// which meant no position ⇒ no channel key ⇒ proximity-only ⇒ silence, in both
/// directions. The game now comes from the connection's authenticated certificate
/// CN, so channel membership resolves without any position, and only the proximity
/// branch still requires coordinates.
///
/// ## Assertion strategy
///
/// Deliberately never calls `server.update_positions(...)`. That omission IS the
/// test — every other audio scenario feeds positions, which is why this regression
/// was invisible to the suite. Transport fidelity (`bob_from_quic == alice_sent`)
/// plus heard experience (rms and per-note Goertzel energy for C4/E4/G4), matching
/// `same_channel`.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn channel_members_hear_each_other_before_joining_the_game() {
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

    // Alice spawns first so she creates "shared-voice"; Bob's Connector then finds it
    // via list_channels and joins the same channel id.
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "shared-voice");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects + joins shared-voice");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "shared-voice");
    bob.await_connected(Duration::from_secs(30))
        .expect("Bob connects + joins shared-voice");

    // NO update_positions call: neither player has ever reported a position, so the
    // server's player_cache holds nothing for either of them and every AudioFrame
    // keeps `sender: None`. Adding a position here would silently convert this into
    // a duplicate of `same_channel` and stop testing anything.

    let probe = Signal::musical_probe(48_000);

    let alice_probe = probe.clone();
    let feed_handle = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe, 48_000);
        alice
    });

    let captured = bob.collect_captured(Duration::from_millis(3_600));

    let alice = feed_handle.join().expect("feed thread panicked");

    let (alice_sent, _, _) = alice.stats();
    let (_, bob_from_quic, bob_received) =
        bob.await_transport_frames(alice_sent, Duration::from_secs(5));

    alice.shutdown();
    bob.shutdown();

    let mono = Signal::to_mono(&captured);
    let rms = Signal::rms(&mono);
    let xcorr = Signal::xcorr_peak(&probe, &mono);

    let frac_c4 = Signal::tone_energy_fraction(&mono, 48_000, 261.63);
    let frac_e4 = Signal::tone_energy_fraction(&mono, 48_000, 329.63);
    let frac_g4 = Signal::tone_energy_fraction(&mono, 48_000, 392.00);

    eprintln!(
        "[channel_without_position] captured_samples={} mono_samples={} \
         rms={:.6} xcorr={:.4} \
         frac_c4={:.4} frac_e4={:.4} frac_g4={:.4} \
         alice_frames_sent={} bob_frames_from_quic={} bob_frames_into_jitter_buffer={}",
        captured.len(),
        mono.len(),
        rms,
        xcorr,
        frac_c4,
        frac_e4,
        frac_g4,
        alice_sent,
        bob_from_quic,
        bob_received,
    );

    assert!(
        alice_sent > 0,
        "FAIL — Alice sent zero AudioFrame datagrams to QUIC: the input pipeline never \
         produced frames",
    );
    assert_eq!(
        bob_from_quic, alice_sent,
        "FAIL — Alice sent {alice_sent} AudioFrame datagrams but Bob received \
         {bob_from_quic} from QUIC. With no position data the router must fall back to \
         the authenticated certificate identity for the channel key; check whether \
         route_audio_frame logs IN_CHANNEL for this pair, or whether it silently \
         dropped to the proximity branch",
    );

    assert!(
        rms > 0.005,
        "FAIL — Bob is silent (rms={rms:.6}) even though both players are in the same \
         channel: channel membership is being gated on position data again",
    );

    assert!(
        frac_c4 > 0.02,
        "FAIL — C4 tone energy missing (frac_c4={frac_c4:.4}): 261.63 Hz did not arrive",
    );
    assert!(
        frac_e4 > 0.02,
        "FAIL — E4 tone energy missing (frac_e4={frac_e4:.4}): 329.63 Hz did not arrive",
    );
    assert!(
        frac_g4 > 0.02,
        "FAIL — G4 tone energy missing (frac_g4={frac_g4:.4}): 392.00 Hz did not arrive",
    );
}
