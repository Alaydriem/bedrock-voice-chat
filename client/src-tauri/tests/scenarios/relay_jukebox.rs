use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::jukebox_fixture::{BM_NOTES, C_NOTES, JukeboxFixture};
use crate::harness::note_energy::NoteEnergy;
use crate::harness::protocol_matrix::ProtocolMatrix;
use crate::harness::proxy_scale::ALICE;
use crate::harness::relay_world::{ActorSpec, RelayWorld, Srv};
use crate::scenarios::relay_voice::DELIVERY_FLOOR;

const NEAR: (f32, f32, f32) = (0.0, 64.0, 0.0);
const BLOCK: (i32, i32, i32) = (0, 64, 0);
// Two jukebox sites far enough apart that a player at one is out of broadcast
// range of the other (positional isolation).
const SITE_1: (i32, i32, i32) = (0, 64, 0);
const SITE_2: (i32, i32, i32) = (10_000, 64, 0);

/// server A plays a jukebox; both players (Alice@A local, Bob@B relayed) are
/// in range and both hear it; an eject stops playback within one loop.
///
/// The emitting player's server is the fulfiller: A synthesizes the track and
/// tags the synthetic player with `relay_world_uuid`, so the frames relay to B.
/// Alice and Bob share no voice server and no channel — Bob hearing the track
/// proves the relay carried it. Cross-server delivery is gated on the relayed
/// listener receiving at least `DELIVERY_FLOOR` × the local listener's frames.
#[tokio::test(flavor = "multi_thread")]
async fn relay_jukebox_a_emits_both_hear_then_eject_stops() {
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

        let positions = [
            ("Alice", NEAR.0, NEAR.1, NEAR.2),
            ("Bob", NEAR.0, NEAR.1, NEAR.2),
        ];
        let converged = w
            .converge_link(
                "Alice",
                "Bob",
                &positions,
                &ALICE.voice(1),
                Duration::from_secs(45),
            )
            .await;
        assert!(converged > 0, "[{v}] peer link established before jukebox");

        let fixture_dir = tempfile::tempdir().expect("fixture dir");
        // A long (6-loop) track so the eject below lands with most of the track
        // still ahead — natural end cannot masquerade as an eject-stop.
        let wav = JukeboxFixture::bm_wav(fixture_dir.path(), "track", 6);
        let (audio_id, _duration_ms) = w
            .proc("Alice")
            .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(30))
            .expect("Alice uploads track to server A");

        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Bob").drain_captured();
        let (_, a_fq0, _) = w.proc("Alice").stats();
        let (_, b_fq0, _) = w.proc("Bob").stats();

        w.play_sound(
            "Alice",
            &audio_id,
            BLOCK.0,
            BLOCK.1,
            BLOCK.2,
            "minecraft:overworld",
        )
        .await;

        // Measure delivery over a fixed mid-track window (the track is far from
        // ending). "Fully hear" is all three notes present; the strict delivery
        // floor is asserted by the B-emits case over a full-track capture (a
        // partial window is skewed by the relayed listener's propagation lag).
        let probe = Duration::from_millis(2_000);
        let cap_a = w.proc("Alice").collect_captured(probe);
        let cap_b = w.proc("Bob").drain_captured();
        let (_, a_fq1, _) = w.proc("Alice").stats();
        let (_, b_fq1, _) = w.proc("Bob").stats();

        let a_fq_d = a_fq1 - a_fq0;
        let b_during = b_fq1 - b_fq0;
        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);
        eprintln!(
            "[relay/C1 {v}] a_fq_d={a_fq_d} b_during={b_during} has_a={} has_b={}",
            NoteEnergy::all_present(&mono_a, &BM_NOTES),
            NoteEnergy::all_present(&mono_b, &BM_NOTES)
        );

        assert!(
            NoteEnergy::all_present(&mono_a, &BM_NOTES),
            "[{v}] Alice (local) hears the Bm track"
        );
        assert!(
            NoteEnergy::all_present(&mono_b, &BM_NOTES),
            "[{v}] Bob (relayed) hears the Bm track across servers"
        );
        assert!(
            a_fq_d > 0 && b_during > 0,
            "[{v}] both receive jukebox frames (relay live)"
        );

        // Eject mid-track, then measure relayed frames over an EQUAL window. If the
        // eject propagated across the relay, the relayed delivery rate collapses;
        // a jukebox that ignored eject would keep delivering at the during-play
        // rate (so the rate comparison fails it regardless of natural track end).
        w.eject("Alice", BLOCK.0, BLOCK.1, BLOCK.2, "minecraft:overworld")
            .await;
        let (_, b_at_eject, _) = w.proc("Bob").stats();
        std::thread::sleep(probe);
        let (_, b_after_eject, _) = w.proc("Bob").stats();
        let b_after = b_after_eject - b_at_eject;
        eprintln!("[relay/C1 {v}] eject b_during={b_during} b_after={b_after}");

        w.shutdown();

        assert!(
            (b_after as f64) * 3.0 < b_during as f64,
            "[{v}] eject must collapse relayed delivery — b_after={b_after} not << b_during={b_during}"
        );
    }
}

/// Server B plays a jukebox; both players hear it (symmetric to the A-emits
/// case). Proves the fulfiller direction is symmetric — frames relay B→A as well
/// as A→B.
#[tokio::test(flavor = "multi_thread")]
async fn relay_jukebox_b_emits_both_hear() {
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

        let positions = [
            ("Alice", NEAR.0, NEAR.1, NEAR.2),
            ("Bob", NEAR.0, NEAR.1, NEAR.2),
        ];
        let converged = w
            .converge_link(
                "Alice",
                "Bob",
                &positions,
                &ALICE.voice(1),
                Duration::from_secs(45),
            )
            .await;
        assert!(converged > 0, "[{v}] peer link established before jukebox");

        let fixture_dir = tempfile::tempdir().expect("fixture dir");
        let wav = JukeboxFixture::bm_wav(fixture_dir.path(), "track", 3);
        let (audio_id, duration_ms) = w
            .proc("Bob")
            .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(30))
            .expect("Bob uploads track to server B");

        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Bob").drain_captured();
        let (_, a_fq0, _) = w.proc("Alice").stats();
        let (_, b_fq0, _) = w.proc("Bob").stats();

        w.play_sound(
            "Bob",
            &audio_id,
            BLOCK.0,
            BLOCK.1,
            BLOCK.2,
            "minecraft:overworld",
        )
        .await;

        // Full-track capture so the relayed listener's propagation lag amortizes,
        // making the strict cross-server delivery floor meaningful.
        let window = Duration::from_millis(duration_ms as u64 + 1_500);
        let cap_b = w.proc("Bob").collect_captured(window);
        let cap_a = w.proc("Alice").drain_captured();
        let (_, a_fq1, _) = w.proc("Alice").stats();
        let (_, b_fq1, _) = w.proc("Bob").stats();

        let a_fq_d = a_fq1 - a_fq0;
        let b_fq_d = b_fq1 - b_fq0;
        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);
        eprintln!(
            "[relay/C2 {v}] a_fq_d={a_fq_d} b_fq_d={b_fq_d} has_a={} has_b={}",
            NoteEnergy::all_present(&mono_a, &BM_NOTES),
            NoteEnergy::all_present(&mono_b, &BM_NOTES)
        );

        w.shutdown();

        assert!(
            NoteEnergy::all_present(&mono_b, &BM_NOTES),
            "[{v}] Bob (local) hears the Bm track"
        );
        assert!(
            NoteEnergy::all_present(&mono_a, &BM_NOTES),
            "[{v}] Alice (relayed) hears the Bm track across servers"
        );
        assert!(
            a_fq_d as f64 >= DELIVERY_FLOOR * b_fq_d as f64,
            "[{v}] relayed delivery floor B→A: a_fq_d={a_fq_d} < floor·b_fq_d={}",
            DELIVERY_FLOOR * b_fq_d as f64
        );
    }
}

/// two simultaneous jukeboxes far apart — each listener hears only their own.
///
/// Alice@A near SITE_1 plays scale S1 (Bm); Bob@B near SITE_2 plays S2 (A-major);
/// the sites are out of broadcast range of each other. Each jukebox relays to the
/// peer server, but the far listener is proximity-gated out. The peer link is
/// converged in range first (so "silent of the other" is gated on a live pipe,
/// not a dead one), then the players move to their distant sites.
#[tokio::test(flavor = "multi_thread")]
async fn relay_jukebox_two_simultaneous_each_hears_only_their_own() {
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

        // Converge the link with both in range, so a missing relay can't be
        // confused with proximity isolation below.
        let near = [("Alice", NEAR.0, NEAR.1, NEAR.2), ("Bob", 1.0, 64.0, 0.0)];
        let converged = w
            .converge_link(
                "Alice",
                "Bob",
                &near,
                &ALICE.voice(1),
                Duration::from_secs(45),
            )
            .await;
        assert!(
            converged > 0,
            "[{v}] peer link established before jukeboxes"
        );

        let dir = tempfile::tempdir().expect("fixture dir");
        let (id1, dur1) = w
            .proc("Alice")
            .upload_audio(
                JukeboxFixture::bm_wav(dir.path(), "s1", 3)
                    .to_str()
                    .unwrap(),
                "minecraft",
                Duration::from_secs(30),
            )
            .expect("Alice uploads S1 to A");
        let (id2, _dur2) = w
            .proc("Bob")
            .upload_audio(
                JukeboxFixture::c_major_wav(dir.path(), "s2", 3)
                    .to_str()
                    .unwrap(),
                "minecraft",
                Duration::from_secs(30),
            )
            .expect("Bob uploads S2 to B");

        // Move to distant sites (out of each other's broadcast range).
        let apart = [
            ("Alice", SITE_1.0 as f32, SITE_1.1 as f32, SITE_1.2 as f32),
            ("Bob", SITE_2.0 as f32, SITE_2.1 as f32, SITE_2.2 as f32),
        ];
        w.pump(&apart, 6, Duration::from_millis(120)).await;

        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Bob").drain_captured();
        let (_, a_fq0, _) = w.proc("Alice").stats();
        let (_, b_fq0, _) = w.proc("Bob").stats();

        w.play_sound(
            "Alice",
            &id1,
            SITE_1.0,
            SITE_1.1,
            SITE_1.2,
            "minecraft:overworld",
        )
        .await;
        w.play_sound(
            "Bob",
            &id2,
            SITE_2.0,
            SITE_2.1,
            SITE_2.2,
            "minecraft:overworld",
        )
        .await;

        let window = Duration::from_millis((dur1 / 2) as u64 + 700);
        let cap_a = w.proc("Alice").collect_captured(window);
        let cap_b = w.proc("Bob").drain_captured();
        let (_, a_fq1, _) = w.proc("Alice").stats();
        let (_, b_fq1, _) = w.proc("Bob").stats();

        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);
        eprintln!(
            "[relay/C3 {v}] a_fq_d={} b_fq_d={} a_has_s1={} a_silent_s2={} b_has_s2={} b_silent_s1={}",
            a_fq1 - a_fq0,
            b_fq1 - b_fq0,
            NoteEnergy::all_present(&mono_a, &BM_NOTES),
            NoteEnergy::all_absent(&mono_a, &C_NOTES),
            NoteEnergy::all_present(&mono_b, &C_NOTES),
            NoteEnergy::all_absent(&mono_b, &BM_NOTES),
        );
        // Per-bin fractions: distinguishes real cross-delivery (all three of the
        // other scale's notes present) from spectral bleed (energy only in bins
        // near the local scale's harmonics).
        for (who, mono) in [("alice", &mono_a), ("bob", &mono_b)] {
            let f = |freq: f32| Signal::tone_energy_fraction(mono, 48_000, freq);
            eprintln!(
                "[relay/C3 {v}] {who} bm(B3={:.4} D4={:.4} F#4={:.4}) c(C6={:.4} E6={:.4} G6={:.4})",
                f(BM_NOTES[0]),
                f(BM_NOTES[1]),
                f(BM_NOTES[2]),
                f(C_NOTES[0]),
                f(C_NOTES[1]),
                f(C_NOTES[2]),
            );
        }

        w.shutdown();

        // Each listener's OWN jukebox frames prove a live pipe (not a dead relay).
        assert!(
            a_fq1 - a_fq0 > 0,
            "[{v}] Alice receives her local jukebox frames"
        );
        assert!(
            b_fq1 - b_fq0 > 0,
            "[{v}] Bob receives his local jukebox frames"
        );
        assert!(
            NoteEnergy::all_present(&mono_a, &BM_NOTES),
            "[{v}] Alice hears her own S1"
        );
        assert!(
            NoteEnergy::all_absent(&mono_a, &C_NOTES),
            "[{v}] Alice does NOT hear Bob's S2 (out of range)"
        );
        assert!(
            NoteEnergy::all_present(&mono_b, &C_NOTES),
            "[{v}] Bob hears his own S2"
        );
        assert!(
            NoteEnergy::all_absent(&mono_b, &BM_NOTES),
            "[{v}] Bob does NOT hear Alice's S1 (out of range)"
        );
    }
}

/// fetch-on-miss — emit on the server that LACKS the file. Server A triggers
/// the jukebox but the track lives only on server B; A discovers B holds it over
/// the peer link (`AudioQuery`→`AudioAvailable`), HTTP-pulls it, and plays it
/// tagged with `relay_world_uuid`. Both in-range players hear it. This is the only
/// test of the peer-link discovery + HTTP pull.
#[tokio::test(flavor = "multi_thread")]
async fn relay_jukebox_fetch_on_miss_pulls_peer_file() {
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

        let positions = [
            ("Alice", NEAR.0, NEAR.1, NEAR.2),
            ("Bob", NEAR.0, NEAR.1, NEAR.2),
        ];
        let converged = w
            .converge_link(
                "Alice",
                "Bob",
                &positions,
                &ALICE.voice(1),
                Duration::from_secs(45),
            )
            .await;
        assert!(
            converged > 0,
            "[{v}] peer link established before fetch-on-miss"
        );

        // The track exists ONLY on server B.
        let fixture_dir = tempfile::tempdir().expect("fixture dir");
        let wav = JukeboxFixture::bm_wav(fixture_dir.path(), "track", 3);
        let (audio_id, duration_ms) = w
            .proc("Bob")
            .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(30))
            .expect("Bob uploads track to server B only");

        let _ = w.proc("Alice").drain_captured();
        let _ = w.proc("Bob").drain_captured();
        let (_, a_fq0, _) = w.proc("Alice").stats();
        let (_, b_fq0, _) = w.proc("Bob").stats();

        // Trigger on server A, which lacks the file: A must discover + pull from B.
        w.play_sound(
            "Alice",
            &audio_id,
            BLOCK.0,
            BLOCK.1,
            BLOCK.2,
            "minecraft:overworld",
        )
        .await;

        let window = Duration::from_millis((duration_ms / 2) as u64 + 1_500);
        let cap_a = w.proc("Alice").collect_captured(window);
        let cap_b = w.proc("Bob").drain_captured();
        let (_, a_fq1, _) = w.proc("Alice").stats();
        let (_, b_fq1, _) = w.proc("Bob").stats();

        let a_fq_d = a_fq1 - a_fq0;
        let b_fq_d = b_fq1 - b_fq0;
        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);
        eprintln!(
            "[relay/C4 {v}] a_fq_d={a_fq_d} b_fq_d={b_fq_d} has_a={} has_b={}",
            NoteEnergy::all_present(&mono_a, &BM_NOTES),
            NoteEnergy::all_present(&mono_b, &BM_NOTES)
        );

        w.shutdown();

        assert!(
            NoteEnergy::all_present(&mono_a, &BM_NOTES),
            "[{v}] Alice@A hears the track A fetched from B over HTTP"
        );
        assert!(
            NoteEnergy::all_present(&mono_b, &BM_NOTES),
            "[{v}] Bob@B hears the track (local fulfiller) "
        );
        assert!(
            a_fq_d > 0 && b_fq_d > 0,
            "[{v}] both receive jukebox frames"
        );
    }
}
