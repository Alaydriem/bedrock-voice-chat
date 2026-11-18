use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::reported_position::ReportedPosition;
use crate::harness::server::EmbeddedServer;

/// A Bedrock player who joins a Java server through Floodgate is heard, and hears, as
/// themselves — for every shape of gamertag Floodgate can mangle.
///
/// ## What this proves
///
/// Real clients, the real server library, and positions through `bvc_update_positions`,
/// the FFI path an embedded Java mod uses. Each scenario plays the mod upgrade: first the
/// roster a beta.27 mod sends (`.Player_One` plus the gamertag as `alternative_identity`),
/// then the current mod's (`Player One`), on one server with the same clients.
///
/// * The roster alone registers the player under their gamertag, and nothing under the
///   Java name. Checked with `player_exists` before `login_code`, which would otherwise
///   create the record it is meant to find.
/// * Each speaker has a frequency: the Floodgate player 440 Hz, the listener 660 Hz, a
///   decoy holding a colliding name 880 Hz. Each side must hear the other's, and the
///   listener must not hear the decoy, who stands far away.
/// * Phase B moves everyone 20 000 blocks. A position entry left over from Phase A — the
///   position cache outlives this scenario — is then out of range and carries nothing.
///
/// Requires the server cdylib and the e2e binary (`mise run test-client` builds both).
struct GateCase {
    id: &'static str,
    gamertag: &'static str,
    beta27: (&'static str, Option<&'static str>),
    new: (&'static str, Option<&'static str>),
    uuid: &'static str,
    // A second player whose name collides with this one's Java name: (name, uuid).
    decoy: Option<(&'static str, &'static str)>,
}

const FLOODGATE_UUID: &str = "00000000-0000-0000-0009-01f2a3b4c5d6";
const FLOODGATE_UUID_2: &str = "00000000-0000-0000-0009-01f2a3b4c5d7";
const JAVA_UUID: &str = "3c9a1b77-2d44-4e18-8b6c-000000000002";
const JAVA_UUID_2: &str = "3c9a1b77-2d44-4e18-8b6c-000000000003";

const SAMPLE_RATE: u32 = 48_000;
const TONE_SECS: f32 = 1.5;
const WINDOW: Duration = Duration::from_millis(2_500);
const PLAYER_HZ: f32 = 440.0;
const LISTENER_HZ: f32 = 660.0;
const DECOY_HZ: f32 = 880.0;
const LISTENER: &str = "Listener";

// Cumulative transport counters taken at the end of a phase, so the next phase measures
// only its own frames.
#[derive(Default, Clone, Copy)]
struct Counters {
    player_sent: u64,
    player_from_quic: u64,
    listener_sent: u64,
    listener_from_quic: u64,
    decoy_from_quic: u64,
}

struct FloodgateGate;

impl FloodgateGate {
    fn push(server: &EmbeddedServer, roster: &[ReportedPosition]) {
        for _ in 0..5 {
            server.update_reported_positions(roster);
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    fn roster(
        case: &GateCase,
        wire: (&str, Option<&str>),
        player_x: f32,
        listener_x: f32,
        decoy_x: f32,
    ) -> Vec<ReportedPosition> {
        let mut roster = vec![
            ReportedPosition::new(wire.0, wire.1, Some(case.uuid), player_x),
            ReportedPosition::new(LISTENER, None, None, listener_x),
        ];
        if let Some((name, uuid)) = case.decoy {
            roster.push(ReportedPosition::new(name, None, Some(uuid), decoy_x));
        }
        roster
    }

    // The listener's feed lists the gamertag within `max_distance` blocks, and never the
    // Java name.
    async fn assert_feed(server: &EmbeddedServer, case: &GateCase, phase: &str, java_name: &str) {
        let identity = format!("minecraft:{}", case.gamertag);
        let java_identity = format!("minecraft:{java_name}");
        let snapshots = server
            .position_snapshots(LISTENER, 1, Duration::from_secs(5))
            .await;
        let snapshot = snapshots
            .first()
            .unwrap_or_else(|| panic!("{} {phase}: the listener's feed carried no entry", case.id));

        let entry = snapshot.positions.iter().find(|p| p.name == identity);
        assert!(
            entry.is_some_and(|e| e.distance <= 2),
            "{} {phase}: the feed does not place {identity} beside the listener: {:?}",
            case.id,
            snapshot.positions
        );
        if java_name != case.gamertag {
            assert!(
                snapshot.positions.iter().all(|p| p.name != java_identity),
                "{} {phase}: the feed names the Java name {java_identity}: {:?}",
                case.id,
                snapshot.positions
            );
        }
    }

    fn phase(
        case: &GateCase,
        phase: &str,
        player: &ClientProc,
        listener: &ClientProc,
        decoy: Option<&ClientProc>,
        before: Counters,
    ) -> Counters {
        let _ = player.drain_captured();
        let _ = listener.drain_captured();

        player.feed_tone(&Signal::chirp(SAMPLE_RATE, TONE_SECS, PLAYER_HZ, PLAYER_HZ), SAMPLE_RATE);
        listener.feed_tone(
            &Signal::chirp(SAMPLE_RATE, TONE_SECS, LISTENER_HZ, LISTENER_HZ),
            SAMPLE_RATE,
        );
        if let Some(decoy) = decoy {
            decoy.feed_tone(&Signal::chirp(SAMPLE_RATE, TONE_SECS, DECOY_HZ, DECOY_HZ), SAMPLE_RATE);
        }
        std::thread::sleep(WINDOW);
        let heard_by_listener = Signal::to_mono(&listener.drain_captured());
        let heard_by_player = Signal::to_mono(&player.drain_captured());

        let (player_sent, _, _) = player.stats();
        let (listener_sent, _, _) = listener.stats();
        let (_, listener_from_quic, _) = listener.await_transport_frames(
            before.listener_from_quic + (player_sent - before.player_sent),
            Duration::from_secs(5),
        );
        let (_, player_from_quic, _) = player.await_transport_frames(
            before.player_from_quic + (listener_sent - before.listener_sent),
            Duration::from_secs(5),
        );

        assert!(
            player_sent > before.player_sent,
            "{} {phase}: the Floodgate player sent no frames",
            case.id
        );
        assert_eq!(
            listener_from_quic - before.listener_from_quic,
            player_sent - before.player_sent,
            "{} {phase}: the listener did not receive every frame the Floodgate player sent",
            case.id
        );
        assert_eq!(
            player_from_quic - before.player_from_quic,
            listener_sent - before.listener_sent,
            "{} {phase}: the Floodgate player did not receive every frame the listener sent",
            case.id
        );

        let player_tone = Signal::tone_energy_fraction(&heard_by_listener, SAMPLE_RATE, PLAYER_HZ);
        let listener_tone = Signal::tone_energy_fraction(&heard_by_player, SAMPLE_RATE, LISTENER_HZ);
        assert!(
            player_tone > 0.02,
            "{} {phase}: the listener did not hear the Floodgate player's 440 Hz ({player_tone:.4})",
            case.id
        );
        assert!(
            listener_tone > 0.02,
            "{} {phase}: the Floodgate player did not hear the listener's 660 Hz ({listener_tone:.4})",
            case.id
        );

        let mut decoy_from_quic = before.decoy_from_quic;
        if let Some(decoy) = decoy {
            let decoy_tone = Signal::tone_energy_fraction(&heard_by_listener, SAMPLE_RATE, DECOY_HZ);
            assert!(
                decoy_tone < 0.005,
                "{} {phase}: the listener heard the far-away decoy's 880 Hz ({decoy_tone:.4})",
                case.id
            );
            decoy_from_quic = decoy.stats().1;
            assert_eq!(
                decoy_from_quic, before.decoy_from_quic,
                "{} {phase}: the far-away decoy received audio",
                case.id
            );
        }

        Counters {
            player_sent,
            player_from_quic,
            listener_sent,
            listener_from_quic,
            decoy_from_quic,
        }
    }

    async fn run(case: GateCase) {
        let data_dir = tempfile::tempdir().expect("create temp data dir");
        let rocket_port = EmbeddedServer::free_port_tcp();
        let quic_port = EmbeddedServer::free_port_udp();
        let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
        let certs_path = data_dir.path().join("certificates");
        let lib = EmbeddedServer::load_library();
        let server =
            EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;
        let url = format!("https://127.0.0.1:{}", server.rocket_port());

        let listener_code = server.login_code(LISTENER);
        let listener = ClientProc::spawn(LISTENER, &listener_code, &url, "listener-solo");
        listener
            .await_connected(Duration::from_secs(30))
            .expect("listener connects");

        // beta.27 roster.
        Self::push(&server, &Self::roster(&case, case.beta27, 0.0, 1.0, 10_000.0));

        assert!(
            server.await_player_exists(case.gamertag, Duration::from_secs(5)),
            "{} registered: the roster did not register {:?}",
            case.id,
            case.gamertag
        );
        // Where the decoy's own name is that Java name, its record is the decoy's, not a
        // stray one; who owns it is pinned by the server's identity service tests.
        let decoy_holds_java_name = case.decoy.is_some_and(|(name, _)| name == case.beta27.0);
        if case.beta27.0 != case.gamertag && !decoy_holds_java_name {
            assert!(
                !server.player_exists(case.beta27.0),
                "{} registered: a record exists under the Java name {:?}",
                case.id,
                case.beta27.0
            );
        }
        Self::assert_feed(&server, &case, "A feed", case.beta27.0).await;

        let decoy = case.decoy.map(|(name, _)| {
            let code = server.login_code(name);
            let decoy = ClientProc::spawn(name, &code, &url, "decoy-solo");
            decoy
                .await_connected(Duration::from_secs(30))
                .expect("decoy connects");
            decoy
        });

        let player_code = server.login_code(case.gamertag);
        let player = ClientProc::spawn(case.gamertag, &player_code, &url, "floodgate-solo");
        player
            .await_connected(Duration::from_secs(30))
            .expect("the Floodgate player connects");
        // A game server reports positions continuously; a client that connected after the
        // last report has no positions to mix with until the next one.
        Self::push(&server, &Self::roster(&case, case.beta27, 0.0, 1.0, 10_000.0));

        let after_a = Self::phase(&case, "A", &player, &listener, decoy.as_ref(), Counters::default());

        // Current mod's roster, everyone moved far from where Phase A left them.
        Self::push(&server, &Self::roster(&case, case.new, 20_001.0, 20_000.0, 30_000.0));
        Self::assert_feed(&server, &case, "B feed", case.new.0).await;
        Self::phase(&case, "B", &player, &listener, decoy.as_ref(), after_a);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn g01_plain_gamertag() {
    FloodgateGate::run(GateCase { id: "G01", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g02_space_with_replace_spaces() {
    FloodgateGate::run(GateCase { id: "G02", gamertag: "Player One", beta27: (".Player_One", Some("Player One")), new: ("Player One", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g03_several_spaces() {
    FloodgateGate::run(GateCase { id: "G03", gamertag: "A B C D", beta27: (".A_B_C_D", Some("A B C D")), new: ("A B C D", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g04_fifteen_characters_with_spaces() {
    FloodgateGate::run(GateCase { id: "G04", gamertag: "Lord Of Pixels1", beta27: (".Lord_Of_Pixels1", Some("Lord Of Pixels1")), new: ("Lord Of Pixels1", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g05_fifteen_characters_fits() {
    FloodgateGate::run(GateCase { id: "G05", gamertag: "LongGamertag123", beta27: ("LongGamertag123", None), new: ("LongGamertag123", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g06_prefix_be_cut_to_sixteen() {
    FloodgateGate::run(GateCase { id: "G06", gamertag: "LongGamertag123", beta27: ("BE_LongGamertag1", Some("LongGamertag123")), new: ("LongGamertag123", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g07_prefix_be() {
    FloodgateGate::run(GateCase { id: "G07", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g08_space_without_replace_spaces() {
    FloodgateGate::run(GateCase { id: "G08", gamertag: "Player One", beta27: ("Player One", None), new: ("Player One", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g09_empty_prefix() {
    FloodgateGate::run(GateCase { id: "G09", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g10_empty_prefix_space_beside_a_java_account() {
    FloodgateGate::run(GateCase { id: "G10", gamertag: "Player One", beta27: ("Player_One", Some("Player One")), new: ("Player One", None), uuid: FLOODGATE_UUID, decoy: Some(("Player_One", JAVA_UUID_2)) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g11_non_latin() {
    FloodgateGate::run(GateCase { id: "G11", gamertag: "テスト ユーザー", beta27: (".テスト_ユーザー", Some("テスト ユーザー")), new: ("テスト ユーザー", None), uuid: FLOODGATE_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g12_linked_java_name() {
    FloodgateGate::run(GateCase { id: "G12", gamertag: "Player One", beta27: ("Linked_Java", Some("Player One")), new: ("Linked_Java", Some("Player One")), uuid: JAVA_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g13_linked_same_name() {
    FloodgateGate::run(GateCase { id: "G13", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: JAVA_UUID, decoy: None }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g14_linked_java_name_beside_a_bedrock_player_of_that_name() {
    FloodgateGate::run(GateCase { id: "G14", gamertag: "LongGamertag123", beta27: ("Builder", Some("LongGamertag123")), new: ("Builder", Some("LongGamertag123")), uuid: JAVA_UUID, decoy: Some(("Builder", FLOODGATE_UUID_2)) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn g17_linked_java_name_ending_with_the_gamertag() {
    FloodgateGate::run(GateCase { id: "G17", gamertag: "Builder", beta27: ("Builder", None), new: ("Builder", Some("Builder")), uuid: JAVA_UUID, decoy: None }).await;
}

// A Java account named Steve stands 10 000 blocks away; the Floodgate player whose
// gamertag is Steve stands beside the listener. If the Java entry won the name, the
// speaker would be out of range and the listener would hear nothing.
#[tokio::test(flavor = "multi_thread")]
async fn k01_java_and_floodgate_named_alike() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();
    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");
    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;
    let url = format!("https://127.0.0.1:{}", server.rocket_port());
    let case = GateCase { id: "K01", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: FLOODGATE_UUID, decoy: None };

    let listener_code = server.login_code(LISTENER);
    let listener = ClientProc::spawn(LISTENER, &listener_code, &url, "listener-solo");
    listener
        .await_connected(Duration::from_secs(30))
        .expect("listener connects");

    let roster = vec![
        ReportedPosition::new("Steve", None, Some(JAVA_UUID_2), 10_000.0),
        ReportedPosition::new("Steve", None, Some(FLOODGATE_UUID), 0.0),
        ReportedPosition::new(LISTENER, None, None, 1.0),
    ];
    FloodgateGate::push(&server, &roster);
    assert!(server.await_player_exists("Steve", Duration::from_secs(5)));
    FloodgateGate::assert_feed(&server, &case, "feed", "Steve").await;

    let player_code = server.login_code("Steve");
    let player = ClientProc::spawn("Steve", &player_code, &url, "floodgate-solo");
    player
        .await_connected(Duration::from_secs(30))
        .expect("the Floodgate player connects");
    FloodgateGate::push(&server, &roster);

    FloodgateGate::phase(&case, "A", &player, &listener, None, Counters::default());
}
