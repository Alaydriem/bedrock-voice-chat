use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::protocol_matrix::ProtocolMatrix;
use crate::harness::proxy_scale::{ALICE, BOB, CAROL, DAVE, Scale};
use crate::harness::proxy_world::ProxyWorld;

/// Proves the full chain stands up over both protocol versions before any audio:
/// real Wry client boots + voice-connects, proxy dials the fake upstream,
/// a downstream BedrockClient attaches (triggering Proxy::accept()), the upstream
/// connection is accepted+mapped by name, StartGame sets the world, and a
/// PlayerAuthInput position drive does not error.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_single_player_attaches_to_upstream() {
    for v in ProtocolMatrix::last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice"]).await;
        // Drive a non-default position via PlayerAuthInput; if the proxy were in
        // transparent-relay mode it would never decode this, so reaching here with
        // the session alive proves inspection mode.
        w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        w.shutdown();
    }
}

/// Two co-located players each emit their own disjoint scale and each hear the
/// other's scale.
///
/// Positions are driven exclusively via PlayerAuthInput through the proxy's fake
/// upstream (`upstream.drive_position`). Alice (C-major) and Bob (A-major) stand
/// 1 block apart — well within the server's broadcast_range gate — so proximity
/// routing must deliver each player's audio to the other.
///
/// Both feeds start immediately after the position loop completes (no background
/// thread — `feed_tone` is fast: it queues frames to stdin without sleeping). A
/// single 4 500 ms sleep covers both feeds. `drain_captured` then snapshots each
/// buffer without a second sleep, keeping total wall time to one capture window.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_two_players_in_range_hear_each_other() {
    for v in ProtocolMatrix::last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice", "Bob"]).await;

        // Drive co-located positions via PlayerAuthInput. Loop so the proxy's
        // ~250 ms heartbeat fires at least once with fresh position data.
        for _ in 0..5 {
            w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
            w.upstream.drive_position("Bob", 1.0, 64.0, 0.0).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // The dashboard's player cards appear off the player_presence Tauri
        // event; Alice's client must observe Bob join at the webview boundary.
        assert!(
            w.proc("Alice")
                .await_ui_event(
                    "player_presence",
                    |p| p.contains("Bob") && p.contains("joined"),
                    Duration::from_secs(10),
                )
                .is_ok(),
            "[{v}] Alice must receive Bob's player_presence render trigger"
        );

        // Queue both feeds back-to-back. feed_tone writes frames to stdin without
        // sleeping; the bin's FrameClock paces them to QUIC at a real 20 ms cadence.
        // Disjoint scales (ALICE = C-major, BOB = A-major) make Scale::hears() unambiguous.
        let alice_pcm = ALICE.voice(2);
        let bob_pcm = BOB.voice(2);
        w.proc("Alice").feed_tone(&alice_pcm, 48_000);
        w.proc("Bob").feed_tone(&bob_pcm, 48_000);

        // One shared capture window covers the ~3 s paced send (≈150 frames × 20 ms)
        // plus QUIC + jitter + decode drain. drain_captured then snapshots both
        // buffers without a second sleep.
        std::thread::sleep(Duration::from_millis(4_500));
        let cap_a = w.proc("Alice").drain_captured();
        let cap_b = w.proc("Bob").drain_captured();

        let (a_sent, a_fq, _) = w.proc("Alice").stats();
        let (b_sent, b_fq, _) = w.proc("Bob").stats();

        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);

        let a_frac_a4 = Signal::tone_energy_fraction(&mono_a, 48_000, 440.00);
        let a_frac_c5 = Signal::tone_energy_fraction(&mono_a, 48_000, 554.37);
        let a_frac_e5 = Signal::tone_energy_fraction(&mono_a, 48_000, 659.25);
        let b_frac_c4 = Signal::tone_energy_fraction(&mono_b, 48_000, 261.63);
        let b_frac_e4 = Signal::tone_energy_fraction(&mono_b, 48_000, 329.63);
        let b_frac_g4 = Signal::tone_energy_fraction(&mono_b, 48_000, 392.00);

        eprintln!(
            "[proxy/B1 {v}] \
             a_sent={a_sent} a_fq={a_fq} b_sent={b_sent} b_fq={b_fq} \
             alice_hears_bob(A4={a_frac_a4:.4} C#5={a_frac_c5:.4} E5={a_frac_e5:.4}) \
             bob_hears_alice(C4={b_frac_c4:.4} E4={b_frac_e4:.4} G4={b_frac_g4:.4})",
        );

        w.shutdown();

        assert!(a_sent > 0 && b_sent > 0, "[{v}] both produce input frames");
        assert!(a_fq > 0 && b_fq > 0, "[{v}] both receive frames from QUIC");
        assert!(
            Scale::hears(&mono_a, BOB),
            "[{v}] Alice hears Bob (A-major triad)"
        );
        assert!(
            Scale::hears(&mono_b, ALICE),
            "[{v}] Bob hears Alice (C-major triad)"
        );
    }
}

/// No-net local shortcut: a bvc:ctl: SELF action driven through the proxy is applied
/// LOCALLY by the `ControlActionsManager` (no server round-trip). Alice sends
/// `bvc:ctl:mute:1` on the proxy PlaySound bus; her own client's input device must
/// become muted, proving the proxy applies self-actions in-process.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_ctl_mute_local() {
    for v in ProtocolMatrix::last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice"]).await;

        for _ in 0..3 {
            w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        assert!(
            w.proc("Alice")
                .await_muted(false, Duration::from_secs(5))
                .is_ok(),
            "[{v}] Alice starts unmuted"
        );

        // Self control action over the no-net path (proxy consumes + applies locally).
        w.upstream.play_ctl("Alice", "mute:1").await;

        assert!(
            w.proc("Alice")
                .await_muted(true, Duration::from_secs(5))
                .is_ok(),
            "[{v}] Alice's input device must be muted by the no-net local shortcut"
        );

        w.shutdown();
    }
}

/// No-net reverse ride: `!bvcs:` state chat is injected ONLY into sessions that
/// proved the BDS world runs the BVC addon (a `bvc:ctl:sync` was decoded) — on
/// a modless server nothing would cancel the chat, so an unarmed ride would
/// broadcast the player's audio state publicly. Once armed, the sync answers
/// with a snapshot ride and subsequent local changes keep riding.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_bvcs_reverse_ride_reports_state_only_after_sync_arms() {
    for v in ProtocolMatrix::last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice"]).await;

        for _ in 0..3 {
            w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // A state change BEFORE any sync must NOT ride — the session is unarmed
        // (this world has shown no evidence of the BVC addon).
        w.upstream.play_ctl("Alice", "mute:1").await;
        assert!(
            w.proc("Alice")
                .await_muted(true, Duration::from_secs(5))
                .is_ok(),
            "[{v}] Alice's input device must be muted by the no-net local shortcut"
        );
        let leaked = w
            .upstream
            .await_bvcs("Alice", |m| m.contains(":q:"), Duration::from_secs(3))
            .await;
        assert!(
            leaked.is_none(),
            "[{v}] no !bvcs: ride may reach an unarmed session (public-chat leak): {leaked:?}"
        );

        // The panel's sync request arms the session and answers with a snapshot.
        w.upstream.play_ctl("Alice", "sync:").await;
        let snapshot = w
            .upstream
            .await_bvcs(
                "Alice",
                |m| m.contains(":q:") && m.contains("m=1"),
                Duration::from_secs(10),
            )
            .await;
        assert!(
            snapshot.is_some(),
            "[{v}] a bvc:ctl:sync request must arm the session and ride a snapshot"
        );

        // Subsequent local changes ride live into the armed session.
        w.upstream.play_ctl("Alice", "mute:0").await;
        let change_ride = w
            .upstream
            .await_bvcs(
                "Alice",
                |m| m.contains(":q:") && m.contains("m=0"),
                Duration::from_secs(10),
            )
            .await;
        assert!(
            change_ride.is_some(),
            "[{v}] a local state change must ride into the armed session"
        );

        w.shutdown();
    }
}

/// A bvc:ctl: control action rides the same PlaySound bus as jukebox commands.
/// This is a NON-DISRUPTION guard: firing control actions through the live proxy
/// mid-conversation must not perturb the voice pipeline. The self action is
/// `mute:0` — it exercises the local-apply path without silencing anyone (Alice
/// is already unmuted); a `mute:1` here would legitimately stop her input, which
/// is the `proxy_ctl_mute_local` scenario's contract, not this one's. It
/// deliberately does NOT assert routing — that contract is pinned by the
/// play_sound unit tests (group -> ServerBound ClientAction, self -> control
/// channel).
#[tokio::test(flavor = "multi_thread")]
async fn proxy_bvc_ctl_does_not_disrupt_proximity_audio() {
    for v in ProtocolMatrix::last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice", "Bob"]).await;

        for _ in 0..5 {
            w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
            w.upstream.drive_position("Bob", 1.0, 64.0, 0.0).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // Fire a self-action and a group-action through the real proxy. Both are
        // consumed by the proxy; neither reaches the client. The self action is a
        // no-op unmute so the pipeline under test keeps flowing.
        w.upstream.play_ctl("Alice", "mute:0").await;
        w.upstream.play_ctl("Alice", "group:create").await;

        let alice_pcm = ALICE.voice(2);
        let bob_pcm = BOB.voice(2);
        w.proc("Alice").feed_tone(&alice_pcm, 48_000);
        w.proc("Bob").feed_tone(&bob_pcm, 48_000);

        std::thread::sleep(Duration::from_millis(4_500));
        let cap_a = w.proc("Alice").drain_captured();
        let cap_b = w.proc("Bob").drain_captured();

        let (a_sent, a_fq, _) = w.proc("Alice").stats();
        let (b_sent, b_fq, _) = w.proc("Bob").stats();

        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);

        eprintln!("[proxy/ctl {v}] a_sent={a_sent} a_fq={a_fq} b_sent={b_sent} b_fq={b_fq}",);

        w.shutdown();

        assert!(a_sent > 0 && b_sent > 0, "[{v}] both produce input frames");
        assert!(a_fq > 0 && b_fq > 0, "[{v}] both receive frames from QUIC");
        assert!(
            Scale::hears(&mono_b, ALICE),
            "[{v}] Bob still hears Alice after control actions (C-major triad)"
        );
        assert!(
            Scale::hears(&mono_a, BOB),
            "[{v}] Alice still hears Bob after control actions (A-major triad)"
        );
    }
}

/// Proximity gate has two sides: in-range delivers, out-of-range does not.
///
/// Phase 1 co-locates Alice and Bob 1 block apart and proves Bob receives QUIC
/// frames from Alice's voice (b_fq_near > 0). This is a false-pass guard: without
/// it, a broken proxy that delivers nothing would trivially "pass" the silence
/// assertion in phase 2.
///
/// Phase 2 drives Bob 10 000 blocks away — far outside the server's
/// broadcast_range gate — then snapshots Bob's cumulative QUIC frame counter,
/// feeds Alice again, and confirms zero incremental frames arrived and Bob's
/// captured buffer is RMS-silent.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_out_of_range_cannot_hear() {
    for v in ProtocolMatrix::last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice", "Bob"]).await;

        // PHASE 1: in-range baseline. Co-locate 1 block apart.
        for _ in 0..5 {
            w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
            w.upstream.drive_position("Bob", 1.0, 64.0, 0.0).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        let alice_pcm = ALICE.voice(2);
        w.proc("Alice").feed_tone(&alice_pcm, 48_000);
        std::thread::sleep(Duration::from_millis(4_500));
        // Drain to clear Bob's buffer before phase 2; only the frame counter matters here.
        let _cap_near = w.proc("Bob").drain_captured();
        let (_, b_fq_near, _) = w.proc("Bob").stats();

        eprintln!("[proxy/B2 {v}] phase1 in-range b_fq_near={b_fq_near}");

        // FALSE-PASS GUARD: Bob must have received frames while in range, else the
        // phase-2 silence assertion proves nothing.
        assert!(
            b_fq_near > 0,
            "[{v}] phase1: Bob receives frames from QUIC while in range"
        );

        // PHASE 2: drive Bob far out of range.
        for _ in 0..5 {
            w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
            w.upstream.drive_position("Bob", 10_000.0, 64.0, 0.0).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // Clear Bob's buffer, then snapshot the cumulative QUIC counter BEFORE the
        // second feed. b_fq_far must equal this if no new frames arrive.
        let _ = w.proc("Bob").drain_captured();
        let (_, b_fq_base, _) = w.proc("Bob").stats();

        w.proc("Alice").feed_tone(&alice_pcm, 48_000);
        std::thread::sleep(Duration::from_millis(4_500));
        let cap_far = w.proc("Bob").drain_captured();
        let (_, b_fq_far, _) = w.proc("Bob").stats();

        let rms_far = Signal::rms(&Signal::to_mono(&cap_far));

        eprintln!(
            "[proxy/B2 {v}] phase2 out-of-range b_fq_base={b_fq_base} b_fq_far={b_fq_far} rms_far={rms_far:.6}"
        );

        w.shutdown();

        assert_eq!(
            b_fq_far, b_fq_base,
            "[{v}] out of range: no incremental QUIC frames delivered to Bob"
        );
        assert!(
            rms_far < 0.01,
            "[{v}] out of range: Bob's capture is silent (rms={rms_far:.6})"
        );
    }
}

/// Three co-located players each emit a disjoint scale; every player hears the
/// other two and not a false echo of themselves.
///
/// Alice (C-major), Bob (A-major), Carol (F-major) stand 1 block apart each —
/// all within broadcast_range. Proximity routing must fan each player's audio
/// out to the other two. A single 4 500 ms window covers all three paced sends.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_three_players_in_range_all_hear() {
    for v in ProtocolMatrix::last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice", "Bob", "Carol"]).await;

        for _ in 0..5 {
            w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
            w.upstream.drive_position("Bob", 1.0, 64.0, 0.0).await;
            w.upstream.drive_position("Carol", 2.0, 64.0, 0.0).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        let alice_pcm = ALICE.voice(2);
        let bob_pcm = BOB.voice(2);
        let carol_pcm = CAROL.voice(2);
        w.proc("Alice").feed_tone(&alice_pcm, 48_000);
        w.proc("Bob").feed_tone(&bob_pcm, 48_000);
        w.proc("Carol").feed_tone(&carol_pcm, 48_000);

        std::thread::sleep(Duration::from_millis(4_500));
        let cap_a = w.proc("Alice").drain_captured();
        let cap_b = w.proc("Bob").drain_captured();
        let cap_c = w.proc("Carol").drain_captured();

        let (a_sent, a_fq, _) = w.proc("Alice").stats();
        let (b_sent, b_fq, _) = w.proc("Bob").stats();
        let (c_sent, c_fq, _) = w.proc("Carol").stats();

        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);
        let mono_c = Signal::to_mono(&cap_c);

        eprintln!(
            "[proxy/B3 {v}] \
             a_sent={a_sent} a_fq={a_fq} b_sent={b_sent} b_fq={b_fq} c_sent={c_sent} c_fq={c_fq} \
             alice_hears(bob={} carol={}) \
             bob_hears(alice={} carol={}) \
             carol_hears(alice={} bob={})",
            Scale::hears(&mono_a, BOB),
            Scale::hears(&mono_a, CAROL),
            Scale::hears(&mono_b, ALICE),
            Scale::hears(&mono_b, CAROL),
            Scale::hears(&mono_c, ALICE),
            Scale::hears(&mono_c, BOB),
        );

        w.shutdown();

        assert!(
            a_fq > 0 && b_fq > 0 && c_fq > 0,
            "[{v}] all three receive frames from QUIC"
        );
        assert!(Scale::hears(&mono_a, BOB), "[{v}] Alice hears Bob");
        assert!(Scale::hears(&mono_a, CAROL), "[{v}] Alice hears Carol");
        assert!(Scale::hears(&mono_b, ALICE), "[{v}] Bob hears Alice");
        assert!(Scale::hears(&mono_b, CAROL), "[{v}] Bob hears Carol");
        assert!(Scale::hears(&mono_c, ALICE), "[{v}] Carol hears Alice");
        assert!(Scale::hears(&mono_c, BOB), "[{v}] Carol hears Bob");
    }
}

/// Two isolated proximity clusters: audio fans out within a pair but never
/// crosses the 10 000-block gap between pairs.
///
/// Pair 1 (Alice, Bob) sits at the origin; pair 2 (Carol, Dave) sits 10 000
/// blocks away. One member of each pair speaks — Alice (C-major) for pair 1,
/// Dave (D6-major) for pair 2 — and the pair partner listens: Bob must hear
/// Alice and be silent of Dave; Carol must hear Dave and be silent of Alice.
/// This is the spatial-isolation contract: proximity routing must scope a
/// speaker's audio to its own cluster.
///
/// The cross-pair speakers are Alice and Dave rather than Alice and Carol
/// because those two scales are spectrally disjoint at single-bin Goertzel
/// resolution (C-major's highest note is 392 Hz; D6-major's lowest is 1175 Hz),
/// so a `silent_of` assertion measures genuine cross-pair absence rather than
/// the synthesis/Opus sidelobe floor. Alice (C-major) and Carol (F-major) share
/// a harmonic neighbourhood — Carol's 698 Hz fundamental leaks ~0.7% raw into
/// Alice's 330 Hz bin, which the Opus round-trip pushes past the 1% gate — so
/// pairing them across the gap would test the probe's spectral hygiene, not the
/// router.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_two_isolated_pairs_audio_within_not_across() {
    for v in ProtocolMatrix::last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice", "Bob", "Carol", "Dave"]).await;

        for _ in 0..5 {
            w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
            w.upstream.drive_position("Bob", 1.0, 64.0, 0.0).await;
            w.upstream
                .drive_position("Carol", 10_000.0, 64.0, 0.0)
                .await;
            w.upstream.drive_position("Dave", 10_001.0, 64.0, 0.0).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        let alice_pcm = ALICE.voice(2);
        let dave_pcm = DAVE.voice(2);
        w.proc("Alice").feed_tone(&alice_pcm, 48_000);
        w.proc("Dave").feed_tone(&dave_pcm, 48_000);

        std::thread::sleep(Duration::from_millis(4_500));
        let cap_b = w.proc("Bob").drain_captured();
        let cap_c = w.proc("Carol").drain_captured();

        let (_, b_fq, _) = w.proc("Bob").stats();
        let (_, c_fq, _) = w.proc("Carol").stats();

        let mono_b = Signal::to_mono(&cap_b);
        let mono_c = Signal::to_mono(&cap_c);

        eprintln!(
            "[proxy/B4 {v}] \
             b_fq={b_fq} c_fq={c_fq} \
             bob_hears_alice={} bob_silent_of_dave={} \
             carol_hears_dave={} carol_silent_of_alice={}",
            Scale::hears(&mono_b, ALICE),
            Scale::silent_of(&mono_b, DAVE),
            Scale::hears(&mono_c, DAVE),
            Scale::silent_of(&mono_c, ALICE),
        );

        w.shutdown();

        assert!(
            b_fq > 0 && c_fq > 0,
            "[{v}] both pair listeners receive frames from QUIC"
        );
        assert!(
            Scale::hears(&mono_b, ALICE),
            "[{v}] Bob hears Alice (same pair)"
        );
        assert!(
            Scale::silent_of(&mono_b, DAVE),
            "[{v}] Bob is silent of Dave (other pair)"
        );
        assert!(
            Scale::hears(&mono_c, DAVE),
            "[{v}] Carol hears Dave (same pair)"
        );
        assert!(
            Scale::silent_of(&mono_c, ALICE),
            "[{v}] Carol is silent of Alice (other pair)"
        );
    }
}
