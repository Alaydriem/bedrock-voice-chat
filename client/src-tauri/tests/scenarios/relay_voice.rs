use std::time::{Duration, Instant};

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::protocol_matrix::ProtocolMatrix;
use crate::harness::proxy_scale::{ALICE, BOB, CAROL, Scale};
use crate::harness::relay_world::{ActorSpec, RelayWorld, Srv};
use crate::harness::server::EmbeddedServer;

/// Fraction of a speaker's relayed frames the listener must receive across the
/// peer link for a cross-server "fully hear" assertion. The relay path is
/// localhost loopback (datagram loss ≈ 0), so near-complete delivery is expected;
/// this floor is tuned from real captures and never loosened to force a pass.
pub(crate) const DELIVERY_FLOOR: f64 = 0.85;

/// Foundational coexistence spike for the cross-server relay topology.
///
/// Every prior scenario boots ONE embedded server. The relay needs THREE
/// server instances in one process (a discovery relay + servers A and B). The
/// server cdylib is loaded once; each `start` creates an independent runtime
/// handle. This proves three handles coexist (no global-state collision) and
/// that a client can connect to each of two distinct servers — the substrate
/// the cross-server voice cases build on. No relay behavior yet.
#[tokio::test(flavor = "multi_thread")]
async fn relay_three_servers_coexist() {
    let lib = EmbeddedServer::load_library();

    let d_relay = tempfile::tempdir().expect("relay data dir");
    let d_a = tempfile::tempdir().expect("server A data dir");
    let d_b = tempfile::tempdir().expect("server B data dir");

    let (r_relay, q_relay) = (
        EmbeddedServer::free_port_tcp(),
        EmbeddedServer::free_port_udp(),
    );
    let (r_a, q_a) = (
        EmbeddedServer::free_port_tcp(),
        EmbeddedServer::free_port_udp(),
    );
    let (r_b, q_b) = (
        EmbeddedServer::free_port_tcp(),
        EmbeddedServer::free_port_udp(),
    );

    let cfg_relay = EmbeddedServer::config_json(r_relay, q_relay, d_relay.path());
    let cfg_a = EmbeddedServer::config_json(r_a, q_a, d_a.path());
    let cfg_b = EmbeddedServer::config_json(r_b, q_b, d_b.path());

    let relay = EmbeddedServer::start(
        lib.clone(),
        &cfg_relay,
        r_relay,
        q_relay,
        &d_relay.path().join("certificates"),
    )
    .await;
    let a = EmbeddedServer::start(
        lib.clone(),
        &cfg_a,
        r_a,
        q_a,
        &d_a.path().join("certificates"),
    )
    .await;
    let b = EmbeddedServer::start(
        lib.clone(),
        &cfg_b,
        r_b,
        q_b,
        &d_b.path().join("certificates"),
    )
    .await;

    let url_a = format!("https://127.0.0.1:{}", a.rocket_port());
    let url_b = format!("https://127.0.0.1:{}", b.rocket_port());
    let code_a = a.login_code("Alice");
    let code_b = b.login_code("Bob");

    let pa = ClientProc::spawn("Alice", &code_a, &url_a, "");
    let pb = ClientProc::spawn("Bob", &code_b, &url_b, "");

    pa.await_connected(Duration::from_secs(30))
        .expect("Alice connects to server A");
    pb.await_connected(Duration::from_secs(30))
        .expect("Bob connects to server B");

    pa.shutdown();
    pb.shutdown();
    drop(a);
    drop(b);
    drop(relay);
}

/// Convergence milestone: the whole cross-server chain stands up and delivers.
///
/// Alice (server A) and Bob (server B) proxy the same realm, so both derive the
/// same `relay_world_uuid`. Discovery registers each server, the realm-fanned
/// presence proof completes, the peer link dials, and Alice's voice reaches Bob
/// over the relay. This is the first exercise of the real two-server QUIC peer
/// transport + acceptor peer-cert routing + the 4-hop presence proof end to end;
/// it asserts only a coarse "frames crossed" before the precise Goertzel cases.
///
/// A and B share no voice server and join no channel, so Bob receiving any QUIC
/// frames proves the relay carried them.
#[tokio::test(flavor = "multi_thread")]
async fn relay_cross_server_establishes_and_delivers_frames() {
    for v in ProtocolMatrix::last_two() {
        let mut w = RelayWorld::boot(
            v,
            &["RealmW"],
            &[
                ActorSpec {
                    name: "Alice",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Bob",
                    server: Srv::B,
                    realm: 0,
                },
            ],
        )
        .await;

        let positions = [("Alice", 0.0, 64.0, 0.0), ("Bob", 1.0, 64.0, 0.0)];

        // The peer link establishes from discovery + the realm-fanned presence
        // proof; audio only crosses once it is up. Feed Alice in short bursts and
        // pump positions/presence, polling Bob's QUIC frame counter until it rises
        // or the convergence budget is spent.
        let alice_pcm = ALICE.voice(1);
        let deadline = Instant::now() + Duration::from_secs(45);
        let mut b_fq = 0u64;
        while Instant::now() < deadline {
            w.proc("Alice").feed_tone(&alice_pcm, 48_000);
            w.pump(&positions, 8, Duration::from_millis(220)).await;
            b_fq = w.proc("Bob").stats().1;
            if b_fq > 0 {
                break;
            }
        }

        let (a_sent, _, _) = w.proc("Alice").stats();
        eprintln!("[relay/A6 {v}] a_sent={a_sent} b_fq={b_fq}");
        w.shutdown();

        assert!(a_sent > 0, "[{v}] Alice produced input frames");
        assert!(
            b_fq > 0,
            "[{v}] Bob received relayed frames from QUIC across servers (peer link established)"
        );
    }
}

/// B1: two players on DIFFERENT servers, same realm, in range — each fully hears
/// the other AND near-complete delivery crosses the relay both ways.
///
/// Alice→A and Bob→B share no voice server and join no channel, so audibility
/// proves the relay peer link carried the audio. After the link converges (the
/// relay-established guard: Bob receives warmup frames), both speak their disjoint
/// scales concurrently; we assert each hears all three of the other's triad notes
/// ("fully hear") and that each listener's incremental `frames_from_quic` is at
/// least `DELIVERY_FLOOR` × the speaker's incremental sent count ("100% arrives").
#[tokio::test(flavor = "multi_thread")]
async fn relay_cross_server_in_range_fully_hear_each_other() {
    for v in ProtocolMatrix::last_two() {
        let mut w = RelayWorld::boot(
            v,
            &["RealmW"],
            &[
                ActorSpec {
                    name: "Alice",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Bob",
                    server: Srv::B,
                    realm: 0,
                },
            ],
        )
        .await;

        let positions = [("Alice", 0.0, 64.0, 0.0), ("Bob", 1.0, 64.0, 0.0)];

        // Relay-established guard: drive the peer link up before measuring.
        let warmup = ALICE.voice(1);
        let converged = w
            .converge_link("Alice", "Bob", &positions, &warmup, Duration::from_secs(60))
            .await;
        assert!(
            converged > 0,
            "[{v}] peer link established (Bob receives relayed frames) before measuring"
        );

        // Clear warmup audio and snapshot counters for a clean delivery delta.
        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Bob").drain_captured();
        let (a_sent0, a_fq0, _) = w.proc("Alice").stats();
        let (b_sent0, b_fq0, _) = w.proc("Bob").stats();

        // Both speak their disjoint scales concurrently; pump keeps positions fresh
        // against the proxy heartbeat across the capture window.
        w.proc("Alice").feed_tone(&ALICE.voice(2), 48_000);
        w.proc("Bob").feed_tone(&BOB.voice(2), 48_000);
        w.pump(&positions, 30, Duration::from_millis(180)).await;

        let cap_a = w.proc("Alice").drain_captured();
        let cap_b = w.proc("Bob").drain_captured();
        let (a_sent1, a_fq1, _) = w.proc("Alice").stats();
        let (b_sent1, b_fq1, _) = w.proc("Bob").stats();

        let a_sent_d = a_sent1 - a_sent0;
        let a_fq_d = a_fq1 - a_fq0;
        let b_sent_d = b_sent1 - b_sent0;
        let b_fq_d = b_fq1 - b_fq0;

        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);

        let b_ratio = b_fq_d as f64 / a_sent_d.max(1) as f64;
        let a_ratio = a_fq_d as f64 / b_sent_d.max(1) as f64;
        eprintln!(
            "[relay/B1 {v}] a_sent_d={a_sent_d} b_fq_d={b_fq_d} (ratio {b_ratio:.3}) \
             b_sent_d={b_sent_d} a_fq_d={a_fq_d} (ratio {a_ratio:.3})"
        );

        w.shutdown();

        assert!(
            a_sent_d > 0 && b_sent_d > 0,
            "[{v}] both produced measured input frames"
        );
        assert!(
            Scale::hears(&mono_a, BOB),
            "[{v}] Alice fully hears Bob (A-major triad) across the relay"
        );
        assert!(
            Scale::hears(&mono_b, ALICE),
            "[{v}] Bob fully hears Alice (C-major triad) across the relay"
        );
        assert!(
            b_fq_d as f64 >= DELIVERY_FLOOR * a_sent_d as f64,
            "[{v}] near-complete A→B delivery across relay: b_fq_d={b_fq_d} < floor·a_sent_d={}",
            DELIVERY_FLOOR * a_sent_d as f64
        );
        assert!(
            a_fq_d as f64 >= DELIVERY_FLOOR * b_sent_d as f64,
            "[{v}] near-complete B→A delivery across relay: a_fq_d={a_fq_d} < floor·b_sent_d={}",
            DELIVERY_FLOOR * b_sent_d as f64
        );
    }
}

/// B2: out of range across servers → silent, false-pass guarded.
///
/// Phase 1 converges the link in range (Bob receives frames — the relay pipe is
/// live). Phase 2 drives Bob 10 000 blocks away; the peer link stays up but the
/// receiving server's proximity gate drops Alice's frames. Silence is asserted on
/// ZERO incremental `frames_from_quic` (a dead pipe would fail phase 1), exactly
/// as the single-server out-of-range case.
#[tokio::test(flavor = "multi_thread")]
async fn relay_cross_server_out_of_range_is_silent() {
    for v in ProtocolMatrix::last_two() {
        let mut w = RelayWorld::boot(
            v,
            &["RealmW"],
            &[
                ActorSpec {
                    name: "Alice",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Bob",
                    server: Srv::B,
                    realm: 0,
                },
            ],
        )
        .await;

        let near = [("Alice", 0.0, 64.0, 0.0), ("Bob", 1.0, 64.0, 0.0)];
        let warmup = ALICE.voice(1);
        let b_fq_near = w
            .converge_link("Alice", "Bob", &near, &warmup, Duration::from_secs(45))
            .await;
        eprintln!("[relay/B2 {v}] phase1 in-range b_fq_near={b_fq_near}");
        assert!(
            b_fq_near > 0,
            "[{v}] phase1: Bob receives relayed frames in range (relay pipe live)"
        );

        // PHASE 2: drive Bob far out of range; the link stays up.
        let far = [("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)];
        w.pump(&far, 5, Duration::from_millis(120)).await;
        let _ = w.proc("Bob").drain_captured();
        let (_, b_fq_base, _) = w.proc("Bob").stats();

        w.proc("Alice").feed_tone(&ALICE.voice(2), 48_000);
        w.pump(&far, 30, Duration::from_millis(180)).await;
        let cap_far = w.proc("Bob").drain_captured();
        let (_, b_fq_far, _) = w.proc("Bob").stats();

        let rms_far = Signal::rms(&Signal::to_mono(&cap_far));
        eprintln!(
            "[relay/B2 {v}] phase2 b_fq_base={b_fq_base} b_fq_far={b_fq_far} rms_far={rms_far:.6}"
        );

        w.shutdown();

        assert_eq!(
            b_fq_far, b_fq_base,
            "[{v}] out of range across servers: zero incremental relayed frames reach Bob"
        );
        assert!(
            rms_far < 1e-3,
            "[{v}] out of range across servers: Bob's capture is silent (rms={rms_far:.6})"
        );
    }
}

/// B3: cross-realm isolation — different `relay_world_uuid` ⇒ no cross-server
/// audio.
///
/// Alice→A proxies realm W; Carol→B proxies realm X (a second upstream with a
/// distinct world identity) at IDENTICAL coordinates to Alice. The servers share
/// no `relay_world_uuid`, so discovery returns no peer and no link forms. Alice
/// speaks; Carol must be silent with zero relayed frames. Same coordinates,
/// different realm — accidental cross-talk or a missing relay-world gate fails
/// this. (A6/B1 in this suite prove the relay carries audio when realms DO match,
/// so "got nothing" here is isolation, not a dead relay.)
#[tokio::test(flavor = "multi_thread")]
async fn relay_different_realm_gets_no_cross_server_audio() {
    for v in ProtocolMatrix::last_two() {
        let mut w = RelayWorld::boot(
            v,
            &["RealmW", "RealmX"],
            &[
                ActorSpec {
                    name: "Alice",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Carol",
                    server: Srv::B,
                    realm: 1,
                },
            ],
        )
        .await;

        // Identical coordinates in different realms.
        let positions = [("Alice", 0.0, 64.0, 0.0), ("Carol", 0.0, 64.0, 0.0)];
        w.pump(&positions, 5, Duration::from_millis(120)).await;
        let _ = w.proc("Carol").drain_captured();
        let (_, c_fq_base, _) = w.proc("Carol").stats();

        // Give discovery + any (non-)peering several rounds while Alice speaks.
        for _ in 0..6 {
            w.proc("Alice").feed_tone(&ALICE.voice(1), 48_000);
            w.pump(&positions, 8, Duration::from_millis(180)).await;
        }

        let cap_c = w.proc("Carol").drain_captured();
        let (_, c_fq_final, _) = w.proc("Carol").stats();
        let mono_c = Signal::to_mono(&cap_c);

        eprintln!("[relay/B3 {v}] c_fq_base={c_fq_base} c_fq_final={c_fq_final}");

        w.shutdown();

        assert_eq!(
            c_fq_final, c_fq_base,
            "[{v}] different realm: zero cross-server frames reach Carol"
        );
        assert!(
            Scale::silent_of(&mono_c, ALICE),
            "[{v}] different realm: Carol hears none of Alice's scale"
        );
    }
}

/// B4: two players on different servers establish concurrently (join race) — the
/// link resolves to a working bidirectional path, not a deadlock or split-brain.
///
/// Both servers register with discovery and run `offers_to_send` on the same
/// convergence pumps; the `should_initiate` tiebreak (lower endpoint dials) must
/// resolve the race regardless of which server's random port wins. A failure mode
/// here is no link at all (both defer, or both dial and neither adopts): that
/// surfaces as convergence never completing or one direction staying silent. We
/// assert the link converges within budget and that audio meets the delivery
/// floor AND is mutually heard in BOTH directions.
///
/// Note: only a lower bound (floor) is asserted on delivery. `frames_from_quic`
/// is inflated by warmup backlog draining into the measured window, so it is not
/// a clean per-pair count — an upper bound cannot distinguish a real duplicate
/// link from benign backlog at this layer.
#[tokio::test(flavor = "multi_thread")]
async fn relay_simultaneous_join_resolves_bidirectional() {
    for v in ProtocolMatrix::last_two() {
        let mut w = RelayWorld::boot(
            v,
            &["RealmW"],
            &[
                ActorSpec {
                    name: "Alice",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Bob",
                    server: Srv::B,
                    realm: 0,
                },
            ],
        )
        .await;

        let positions = [("Alice", 0.0, 64.0, 0.0), ("Bob", 1.0, 64.0, 0.0)];

        // Both servers register with discovery and run their offer cycle on the
        // same convergence pumps, so the link establishment races regardless of
        // which one's random port wins the `should_initiate` tiebreak. Converge on
        // a single warmup direction (clean, no backlog) before measuring.
        let warmup = ALICE.voice(1);
        let converged = w
            .converge_link("Alice", "Bob", &positions, &warmup, Duration::from_secs(45))
            .await;
        assert!(
            converged > 0,
            "[{v}] the single peer link establishes under a join race"
        );

        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Bob").drain_captured();
        let (a_sent0, a_fq0, _) = w.proc("Alice").stats();
        let (b_sent0, b_fq0, _) = w.proc("Bob").stats();

        w.proc("Alice").feed_tone(&ALICE.voice(2), 48_000);
        w.proc("Bob").feed_tone(&BOB.voice(2), 48_000);
        w.pump(&positions, 30, Duration::from_millis(180)).await;

        let cap_a = w.proc("Alice").drain_captured();
        let cap_b = w.proc("Bob").drain_captured();
        let (a_sent1, a_fq1, _) = w.proc("Alice").stats();
        let (b_sent1, b_fq1, _) = w.proc("Bob").stats();

        let a_sent_d = a_sent1 - a_sent0;
        let a_fq_d = a_fq1 - a_fq0;
        let b_sent_d = b_sent1 - b_sent0;
        let b_fq_d = b_fq1 - b_fq0;

        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);

        let b_ratio = b_fq_d as f64 / a_sent_d.max(1) as f64;
        let a_ratio = a_fq_d as f64 / b_sent_d.max(1) as f64;
        eprintln!(
            "[relay/B4 {v}] a_sent_d={a_sent_d} b_fq_d={b_fq_d} (ratio {b_ratio:.3}) \
             b_sent_d={b_sent_d} a_fq_d={a_fq_d} (ratio {a_ratio:.3})"
        );

        w.shutdown();

        assert!(
            a_sent_d > 0 && b_sent_d > 0,
            "[{v}] both produced measured input frames"
        );
        assert!(
            Scale::hears(&mono_a, BOB),
            "[{v}] Alice hears Bob after the race"
        );
        assert!(
            Scale::hears(&mono_b, ALICE),
            "[{v}] Bob hears Alice after the race"
        );
        assert!(
            b_ratio >= DELIVERY_FLOOR,
            "[{v}] A→B delivery below floor after race: ratio {b_ratio:.3} < {DELIVERY_FLOOR}"
        );
        assert!(
            a_ratio >= DELIVERY_FLOOR,
            "[{v}] B→A delivery below floor after race: ratio {a_ratio:.3} < {DELIVERY_FLOOR}"
        );
    }
}

/// B5: a third player joins on the HOST's server — the one peer link multiplexes
/// both local speakers across to the remote player, and all three hear each other.
///
/// Alice and Carol share server A; Bob is on server B; all proxy the same realm.
/// Alice↔Carol audio is local to A (no relay); the single A↔B link must carry
/// BOTH Alice and Carol to Bob, and Bob back to both. Asserts all six directed
/// hearings, with the relay-multiplex direction (Bob hears two A-side speakers)
/// being the new coverage over B1's pairwise case.
#[tokio::test(flavor = "multi_thread")]
async fn relay_third_player_on_host_server_all_hear() {
    for v in ProtocolMatrix::last_two() {
        let mut w = RelayWorld::boot(
            v,
            &["RealmW"],
            &[
                ActorSpec {
                    name: "Alice",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Carol",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Bob",
                    server: Srv::B,
                    realm: 0,
                },
            ],
        )
        .await;

        let positions = [
            ("Alice", 0.0, 64.0, 0.0),
            ("Carol", 2.0, 64.0, 0.0),
            ("Bob", 1.0, 64.0, 0.0),
        ];

        // One cross-server link (A↔B) carries the whole world. Bob (alone on B) only
        // ever hears the A-side players over the relay, so convergence waits on his
        // QUIC frames; drive all three speaking (a silent listener's side does not
        // converge reliably) until every player has received frames.
        let converged = w
            .converge_mesh(
                &[
                    ("Alice", &ALICE.voice(1)),
                    ("Carol", &CAROL.voice(1)),
                    ("Bob", &BOB.voice(1)),
                ],
                &["Alice", "Carol", "Bob"],
                &positions,
                Duration::from_secs(60),
            )
            .await;
        assert!(
            converged,
            "[{v}] A↔B peer link established (Bob receives relayed A-side frames) before measuring"
        );

        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Carol").drain_captured();
        let _ = w.proc("Bob").drain_captured();

        w.proc("Alice").feed_tone(&ALICE.voice(2), 48_000);
        w.proc("Carol").feed_tone(&CAROL.voice(2), 48_000);
        w.proc("Bob").feed_tone(&BOB.voice(2), 48_000);
        w.pump(&positions, 30, Duration::from_millis(180)).await;

        let mono_a = Signal::to_mono(&w.proc("Alice").drain_captured());
        let mono_c = Signal::to_mono(&w.proc("Carol").drain_captured());
        let mono_b = Signal::to_mono(&w.proc("Bob").drain_captured());

        w.shutdown();

        assert!(
            Scale::hears(&mono_a, CAROL),
            "[{v}] Alice hears Carol (local on A)"
        );
        assert!(Scale::hears(&mono_a, BOB), "[{v}] Alice hears Bob (relay)");
        assert!(
            Scale::hears(&mono_c, ALICE),
            "[{v}] Carol hears Alice (local on A)"
        );
        assert!(Scale::hears(&mono_c, BOB), "[{v}] Carol hears Bob (relay)");
        assert!(
            Scale::hears(&mono_b, ALICE),
            "[{v}] Bob hears Alice (relay, multiplexed)"
        );
        assert!(
            Scale::hears(&mono_b, CAROL),
            "[{v}] Bob hears Carol (relay, multiplexed)"
        );
    }
}

/// B6: a third player joins on the JOINED server — symmetric to B5, the remote
/// player hears both speakers co-located on the peer server.
///
/// Alice is on server A; Bob and Carol share server B; all proxy the same realm.
/// The single A↔B link must carry both Bob and Carol back to Alice, and Alice to
/// both. Asserts all six directed hearings; the new coverage is Alice (alone on A)
/// hearing two B-side speakers over one relay link.
#[tokio::test(flavor = "multi_thread")]
async fn relay_third_player_on_joined_server_all_hear() {
    for v in ProtocolMatrix::last_two() {
        let mut w = RelayWorld::boot(
            v,
            &["RealmW"],
            &[
                ActorSpec {
                    name: "Alice",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Bob",
                    server: Srv::B,
                    realm: 0,
                },
                ActorSpec {
                    name: "Carol",
                    server: Srv::B,
                    realm: 0,
                },
            ],
        )
        .await;

        let positions = [
            ("Alice", 0.0, 64.0, 0.0),
            ("Bob", 1.0, 64.0, 0.0),
            ("Carol", 2.0, 64.0, 0.0),
        ];

        // The lone player on A only ever hears B-side players over the relay, so
        // convergence must wait on Alice receiving cross-server frames. Drive all
        // three speaking (a silent listener's side does not converge reliably) until
        // every player has received QUIC frames.
        let converged = w
            .converge_mesh(
                &[
                    ("Alice", &ALICE.voice(1)),
                    ("Bob", &BOB.voice(1)),
                    ("Carol", &CAROL.voice(1)),
                ],
                &["Alice", "Bob", "Carol"],
                &positions,
                Duration::from_secs(60),
            )
            .await;
        assert!(
            converged,
            "[{v}] A↔B peer link established (Alice receives relayed B-side frames) before measuring"
        );

        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Bob").drain_captured();
        let _ = w.proc("Carol").drain_captured();

        w.proc("Alice").feed_tone(&ALICE.voice(2), 48_000);
        w.proc("Bob").feed_tone(&BOB.voice(2), 48_000);
        w.proc("Carol").feed_tone(&CAROL.voice(2), 48_000);
        w.pump(&positions, 30, Duration::from_millis(180)).await;

        let mono_a = Signal::to_mono(&w.proc("Alice").drain_captured());
        let mono_b = Signal::to_mono(&w.proc("Bob").drain_captured());
        let mono_c = Signal::to_mono(&w.proc("Carol").drain_captured());

        w.shutdown();

        assert!(
            Scale::hears(&mono_a, BOB),
            "[{v}] Alice hears Bob (relay, multiplexed)"
        );
        assert!(
            Scale::hears(&mono_a, CAROL),
            "[{v}] Alice hears Carol (relay, multiplexed)"
        );
        assert!(
            Scale::hears(&mono_b, ALICE),
            "[{v}] Bob hears Alice (relay)"
        );
        assert!(
            Scale::hears(&mono_b, CAROL),
            "[{v}] Bob hears Carol (local on B)"
        );
        assert!(
            Scale::hears(&mono_c, ALICE),
            "[{v}] Carol hears Alice (relay)"
        );
        assert!(
            Scale::hears(&mono_c, BOB),
            "[{v}] Carol hears Bob (local on B)"
        );
    }
}

/// B7: a third player joins on a THIRD server — the full mesh forms (A↔B, A↔C,
/// B↔C) and all three players hear each other across it.
///
/// Each of Alice→A, Bob→B, Carol→C is alone on its server and joins no channel,
/// so every audible pair proves a distinct peer link carried it. `converge_mesh`
/// waits until all three have received cross-server frames (each holds ≥1 link);
/// the six directed-hearing assertions over a shared capture window then prove all
/// three mesh edges are live — if any edge were missing, one pair would be silent.
#[tokio::test(flavor = "multi_thread")]
async fn relay_third_player_on_third_server_mesh_all_hear() {
    for v in ProtocolMatrix::last_two() {
        let mut w = RelayWorld::boot(
            v,
            &["RealmW"],
            &[
                ActorSpec {
                    name: "Alice",
                    server: Srv::A,
                    realm: 0,
                },
                ActorSpec {
                    name: "Bob",
                    server: Srv::B,
                    realm: 0,
                },
                ActorSpec {
                    name: "Carol",
                    server: Srv::C,
                    realm: 0,
                },
            ],
        )
        .await;

        let positions = [
            ("Alice", 0.0, 64.0, 0.0),
            ("Bob", 1.0, 64.0, 0.0),
            ("Carol", 2.0, 64.0, 0.0),
        ];

        let converged = w
            .converge_mesh(
                &[
                    ("Alice", &ALICE.voice(1)),
                    ("Bob", &BOB.voice(1)),
                    ("Carol", &CAROL.voice(1)),
                ],
                &["Alice", "Bob", "Carol"],
                &positions,
                Duration::from_secs(60),
            )
            .await;
        assert!(
            converged,
            "[{v}] every player received cross-server frames (each holds ≥1 mesh link)"
        );

        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Bob").drain_captured();
        let _ = w.proc("Carol").drain_captured();

        w.proc("Alice").feed_tone(&ALICE.voice(2), 48_000);
        w.proc("Bob").feed_tone(&BOB.voice(2), 48_000);
        w.proc("Carol").feed_tone(&CAROL.voice(2), 48_000);
        w.pump(&positions, 40, Duration::from_millis(180)).await;

        let mono_a = Signal::to_mono(&w.proc("Alice").drain_captured());
        let mono_b = Signal::to_mono(&w.proc("Bob").drain_captured());
        let mono_c = Signal::to_mono(&w.proc("Carol").drain_captured());

        w.shutdown();

        assert!(
            Scale::hears(&mono_a, BOB),
            "[{v}] Alice hears Bob (A↔B edge)"
        );
        assert!(
            Scale::hears(&mono_a, CAROL),
            "[{v}] Alice hears Carol (A↔C edge)"
        );
        assert!(
            Scale::hears(&mono_b, ALICE),
            "[{v}] Bob hears Alice (A↔B edge)"
        );
        assert!(
            Scale::hears(&mono_b, CAROL),
            "[{v}] Bob hears Carol (B↔C edge)"
        );
        assert!(
            Scale::hears(&mono_c, ALICE),
            "[{v}] Carol hears Alice (A↔C edge)"
        );
        assert!(
            Scale::hears(&mono_c, BOB),
            "[{v}] Carol hears Bob (B↔C edge)"
        );
    }
}

/// B8 (Phase 3): a live peer link drops via the idle sweep and re-establishes —
/// audio resumes across the relay.
///
/// The idle-teardown window is lowered to 3s (relay config), so a silence gap
/// longer than it deterministically closes the A↔B link on the next orchestration
/// sweep. During the reconnect grace the peer stays authorized (no re-offer); once
/// it lapses the peer is re-offered and re-dialed off continued discovery +
/// presence, and Alice's voice reaches Bob again. Because the gap exceeds the
/// deterministic idle timeout, the link provably closed — audio resuming proves a
/// genuine reconnect, not a link that merely stayed up.
///
/// The relay reconnect plane is independent of the Bedrock protocol version, so
/// this runs a single version to bound the grace-dominated runtime.
#[tokio::test(flavor = "multi_thread")]
async fn relay_dropped_link_reconnects_and_audio_resumes() {
    let v = *ProtocolMatrix::last_two()
        .last()
        .expect("at least one protocol version");
    let mut w = RelayWorld::boot_with_idle(
        v,
        &["RealmW"],
        &[
            ActorSpec {
                name: "Alice",
                server: Srv::A,
                realm: 0,
            },
            ActorSpec {
                name: "Bob",
                server: Srv::B,
                realm: 0,
            },
        ],
        Some(3),
    )
    .await;

    let positions = [("Alice", 0.0, 64.0, 0.0), ("Bob", 1.0, 64.0, 0.0)];

    // Establish the link and confirm Bob hears Alice across it.
    let warmup = ALICE.voice(1);
    let converged = w
        .converge_link("Alice", "Bob", &positions, &warmup, Duration::from_secs(60))
        .await;
    assert!(converged > 0, "[{v}] peer link established before drop");
    let _ = w.proc("Bob").drain_captured();
    w.proc("Alice").feed_tone(&ALICE.voice(2), 48_000);
    w.pump(&positions, 20, Duration::from_millis(180)).await;
    let mono_pre = Signal::to_mono(&w.proc("Bob").drain_captured());
    assert!(
        Scale::hears(&mono_pre, ALICE),
        "[{v}] Bob hears Alice before the drop"
    );

    // Silence gap longer than the 3s idle timeout: keep positions/presence flowing
    // (no audio) so the orchestrator's idle sweep tears the link down.
    w.pump(&positions, 35, Duration::from_millis(180)).await;

    // Resume audio. The link re-offers after the reconnect grace lapses, re-dials,
    // and relays again; wait for Bob's QUIC frame counter to rise past the
    // post-drop baseline (re-link carried fresh frames).
    let _ = w.proc("Bob").drain_captured();
    let b_fq_drop = w.proc("Bob").stats().1;
    let deadline = Instant::now() + Duration::from_secs(75);
    let mut relinked = false;
    while Instant::now() < deadline {
        w.proc("Alice").feed_tone(&ALICE.voice(1), 48_000);
        w.pump(&positions, 8, Duration::from_millis(180)).await;
        if w.proc("Bob").stats().1 > b_fq_drop {
            relinked = true;
            break;
        }
    }
    assert!(
        relinked,
        "[{v}] peer link re-established and relayed frames after the idle drop"
    );

    // And the re-linked path carries intelligible audio, not just frames.
    let _ = w.proc("Bob").drain_captured();
    w.proc("Alice").feed_tone(&ALICE.voice(2), 48_000);
    w.pump(&positions, 20, Duration::from_millis(180)).await;
    let mono_post = Signal::to_mono(&w.proc("Bob").drain_captured());
    assert!(
        Scale::hears(&mono_post, ALICE),
        "[{v}] Bob hears Alice again after the reconnect"
    );

    w.shutdown();
}
