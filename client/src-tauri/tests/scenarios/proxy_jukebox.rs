use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::jukebox_fixture::{bm_wav, BM_NOTES};
use crate::scenarios::proxy_audio::last_two;
use crate::harness::proxy_world::ProxyWorld;

const NEAR: (f32, f32, f32) = (0.0, 64.0, 0.0);
const FAR: (f32, f32, f32) = (10_000.0, 64.0, 0.0);
const JUKEBOX_BLOCK: (i32, i32, i32) = (0, 64, 0);

fn has_all_notes(mono: &[f32]) -> bool {
    BM_NOTES.iter().all(|&f| Signal::tone_energy_fraction(mono, 48_000, f) > 0.02)
}

/// C1: Two players both in proximity of a jukebox block both hear the inserted
/// track via the proxy PlaySound path; an eject stops playback within one loop.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_jukebox_both_in_range_hear_then_eject_stops() {
    for v in last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice", "Bob"]).await;

        let fixture_dir = tempfile::tempdir().expect("fixture dir");
        let wav = bm_wav(fixture_dir.path(), "track", 3);
        let (audio_id, duration_ms) = w
            .proc("Alice")
            .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(30))
            .expect("Alice uploads track");
        assert!(duration_ms > 0, "uploaded fixture has non-zero duration");

        // Drive both players to NEAR so the proximity gate fires.
        for _ in 0..5 {
            w.upstream.drive_position("Alice", NEAR.0, NEAR.1, NEAR.2).await;
            w.upstream.drive_position("Bob", NEAR.0, NEAR.1, NEAR.2).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // Insert the track via the proxy PlaySound path.
        w.upstream
            .play_sound("Alice", &audio_id, JUKEBOX_BLOCK.0, JUKEBOX_BLOCK.1, JUKEBOX_BLOCK.2, "minecraft:overworld")
            .await;

        // Capture ~half duration so both frames accumulate without waiting for the full track.
        let half_window = Duration::from_millis((duration_ms / 2) as u64 + 500);
        std::thread::sleep(half_window);
        let cap_a = w.proc("Alice").drain_captured();
        let cap_b = w.proc("Bob").drain_captured();

        let (_, a_fq, _) = w.proc("Alice").stats();
        let (_, b_fq, _) = w.proc("Bob").stats();
        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);

        let has_a = has_all_notes(&mono_a);
        let has_b = has_all_notes(&mono_b);

        eprintln!(
            "[proxy/C1 {v}] mid-play a_fq={a_fq} b_fq={b_fq} \
             has_notes_a={has_a} has_notes_b={has_b} \
             rms_a={:.4} rms_b={:.4}",
            Signal::rms(&mono_a),
            Signal::rms(&mono_b),
        );

        assert!(a_fq > 0 && b_fq > 0, "[{v}] C1: both receive frames from QUIC mid-play");
        assert!(has_a, "[{v}] C1: Alice hears all Bm notes");
        assert!(has_b, "[{v}] C1: Bob hears all Bm notes");

        // Snapshot frame counts just before eject.
        let (_, a_fq_at_eject, _) = w.proc("Alice").stats();

        w.upstream
            .eject("Alice", JUKEBOX_BLOCK.0, JUKEBOX_BLOCK.1, JUKEBOX_BLOCK.2, "minecraft:overworld")
            .await;

        // Eject-settle: 1.5 s is enough for the JukeboxEject event to propagate and
        // for the playback task to flush its final in-flight frames.
        std::thread::sleep(Duration::from_millis(1_500));

        let (_, a_fq_final, _) = w.proc("Alice").stats();

        // One-loop frame budget: (duration_ms / repeats) / 20 ms per frame.
        let one_loop_frames = ((duration_ms / 3) as u64) / 20;
        let a_delta = (a_fq_final as u64).saturating_sub(a_fq_at_eject as u64);

        eprintln!(
            "[proxy/C1 {v}] eject a_fq_at_eject={a_fq_at_eject} a_fq_final={a_fq_final} \
             a_delta={a_delta} one_loop_frames={one_loop_frames}"
        );

        w.shutdown();

        assert!(
            a_delta < one_loop_frames,
            "[{v}] C1: post-eject frame delta ({a_delta}) must be < one loop ({one_loop_frames})"
        );
    }
}

/// C2: Alice in range hears the jukebox; Bob at FAR (10 000 blocks) receives
/// zero incremental frames and is RMS-silent.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_jukebox_in_range_hears_out_silent() {
    for v in last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice", "Bob"]).await;

        let fixture_dir = tempfile::tempdir().expect("fixture dir");
        let wav = bm_wav(fixture_dir.path(), "track", 2);
        let (audio_id, duration_ms) = w
            .proc("Alice")
            .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(30))
            .expect("Alice uploads track");

        // Drive Alice near the jukebox, Bob far away.
        for _ in 0..5 {
            w.upstream.drive_position("Alice", NEAR.0, NEAR.1, NEAR.2).await;
            w.upstream.drive_position("Bob", FAR.0, FAR.1, FAR.2).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // Drain Bob's baseline and snapshot the frame counter BEFORE play_sound so
        // the incremental assertion can't be confused by any startup noise.
        let _ = w.proc("Bob").drain_captured();
        let (_, b_fq_base, _) = w.proc("Bob").stats();

        w.upstream
            .play_sound("Alice", &audio_id, JUKEBOX_BLOCK.0, JUKEBOX_BLOCK.1, JUKEBOX_BLOCK.2, "minecraft:overworld")
            .await;

        let window = Duration::from_millis(duration_ms as u64 + 1_000);
        let cap_a = w.proc("Alice").collect_captured(window);
        let cap_b = w.proc("Bob").drain_captured();

        let (_, a_fq, _) = w.proc("Alice").stats();
        let (_, b_fq_after, _) = w.proc("Bob").stats();
        let mono_a = Signal::to_mono(&cap_a);
        let mono_b = Signal::to_mono(&cap_b);
        let rms_b = Signal::rms(&mono_b);

        eprintln!(
            "[proxy/C2 {v}] a_fq={a_fq} has_notes_a={} \
             b_fq_base={b_fq_base} b_fq_after={b_fq_after} rms_b={rms_b:.6}",
            has_all_notes(&mono_a),
        );

        w.shutdown();

        assert!(a_fq > 0, "[{v}] C2: Alice receives frames from QUIC");
        assert!(has_all_notes(&mono_a), "[{v}] C2: Alice hears all Bm notes");
        assert_eq!(
            b_fq_after, b_fq_base,
            "[{v}] C2: Bob (out of range) must receive zero incremental QUIC frames"
        );
        assert!(
            rms_b < 0.01,
            "[{v}] C2: Bob (out of range) must be silent (rms={rms_b:.6})"
        );
    }
}

/// C3a: A single-loop track plays fully; after natural end, no further frames
/// arrive and capture is RMS-silent.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_jukebox_natural_end_returns_to_silence() {
    for v in last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice"]).await;

        let fixture_dir = tempfile::tempdir().expect("fixture dir");
        let wav = bm_wav(fixture_dir.path(), "track", 1);
        let (audio_id, duration_ms) = w
            .proc("Alice")
            .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(20))
            .expect("Alice uploads track");

        for _ in 0..5 {
            w.upstream.drive_position("Alice", NEAR.0, NEAR.1, NEAR.2).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        w.upstream
            .play_sound("Alice", &audio_id, JUKEBOX_BLOCK.0, JUKEBOX_BLOCK.1, JUKEBOX_BLOCK.2, "minecraft:overworld")
            .await;

        // Phase A: capture the full playback and verify the notes are present.
        let cap_play = w
            .proc("Alice")
            .collect_captured(Duration::from_millis(duration_ms as u64 + 1_000));
        let mono_play = Signal::to_mono(&cap_play);
        eprintln!(
            "[proxy/C3a {v}] play phase has_notes={} rms={:.4}",
            has_all_notes(&mono_play),
            Signal::rms(&mono_play),
        );
        assert!(
            has_all_notes(&mono_play),
            "[{v}] C3a: Alice must hear all Bm notes during playback"
        );

        // Let the playback task drain and auto-eject settle.
        std::thread::sleep(Duration::from_millis(1_500));
        let (_, a_fq_baseline, _) = w.proc("Alice").stats();

        // Phase B: fresh capture window — silence, no new frames.
        let cap_after = w.proc("Alice").collect_captured(Duration::from_millis(2_000));
        let (_, a_fq_after, _) = w.proc("Alice").stats();
        let rms_after = Signal::rms(&Signal::to_mono(&cap_after));
        eprintln!(
            "[proxy/C3a {v}] post-end a_fq_baseline={a_fq_baseline} a_fq_after={a_fq_after} \
             rms_after={rms_after:.6}"
        );

        w.shutdown();

        assert_eq!(
            a_fq_after, a_fq_baseline,
            "[{v}] C3a: no jukebox frames must arrive after the song ended"
        );
        assert!(
            rms_after < 0.01,
            "[{v}] C3a: capture must be silent after the song ended (rms={rms_after:.6})"
        );
    }
}

/// C3b: An explicit eject mid-play stops the track; post-eject frame delta is
/// < one loop and a fresh capture is RMS-silent.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_jukebox_explicit_eject_returns_to_silence() {
    for v in last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice"]).await;

        let fixture_dir = tempfile::tempdir().expect("fixture dir");
        // 3 repeats gives plenty of in-flight frames to catch mid-play.
        let wav = bm_wav(fixture_dir.path(), "track", 3);
        let (audio_id, duration_ms) = w
            .proc("Alice")
            .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(30))
            .expect("Alice uploads track");

        for _ in 0..5 {
            w.upstream.drive_position("Alice", NEAR.0, NEAR.1, NEAR.2).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        w.upstream
            .play_sound("Alice", &audio_id, JUKEBOX_BLOCK.0, JUKEBOX_BLOCK.1, JUKEBOX_BLOCK.2, "minecraft:overworld")
            .await;

        // Let ~1 loop play so the has_all_notes guard fires, then eject.
        let per_loop_ms = (duration_ms / 3) as u64;
        std::thread::sleep(Duration::from_millis(per_loop_ms + 300));

        let cap_mid = w.proc("Alice").drain_captured();
        let mono_mid = Signal::to_mono(&cap_mid);
        eprintln!(
            "[proxy/C3b {v}] mid-play has_notes={} rms={:.4}",
            has_all_notes(&mono_mid),
            Signal::rms(&mono_mid),
        );
        assert!(
            has_all_notes(&mono_mid),
            "[{v}] C3b: Alice must hear all Bm notes mid-play before eject"
        );

        let (_, a_fq_at_eject, _) = w.proc("Alice").stats();

        w.upstream
            .eject("Alice", JUKEBOX_BLOCK.0, JUKEBOX_BLOCK.1, JUKEBOX_BLOCK.2, "minecraft:overworld")
            .await;

        // Eject-settle: 2 s covers jitter-buffer drain + Rodio playback flush.
        std::thread::sleep(Duration::from_millis(2_000));

        let (_, a_fq_final, _) = w.proc("Alice").stats();
        let one_loop_frames = per_loop_ms / 20;
        let a_delta = (a_fq_final as u64).saturating_sub(a_fq_at_eject as u64);

        // Drain residual in-flight frames from the eject-settle window, then
        // capture a clean 1 s window that should be pure silence.
        let _ = w.proc("Alice").drain_captured();
        let cap_after = w.proc("Alice").collect_captured(Duration::from_millis(1_000));
        let rms_after = Signal::rms(&Signal::to_mono(&cap_after));

        eprintln!(
            "[proxy/C3b {v}] a_fq_at_eject={a_fq_at_eject} a_fq_final={a_fq_final} \
             a_delta={a_delta} one_loop_frames={one_loop_frames} rms_after={rms_after:.6}"
        );

        w.shutdown();

        assert!(
            a_delta < one_loop_frames,
            "[{v}] C3b: post-eject frame delta ({a_delta}) must be < one loop ({one_loop_frames})"
        );
        assert!(
            rms_after < 0.01,
            "[{v}] C3b: capture must be silent after eject (rms={rms_after:.6})"
        );
    }
}

/// C3c: A track ends with nobody nearby; a late arrival who drives into range
/// afterwards must NOT receive any stale/replayed frames — the jukebox is gone.
///
/// Operator contract: a song that ended with nobody present does NOT replay to a
/// late arrival → late arrival silent. If the observed behavior differs (e.g. a
/// still-active song replays), the test reports the evidence rather than weakening
/// the assertion.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_jukebox_ended_with_nobody_then_late_arrival_silent() {
    for v in last_two() {
        let mut w = ProxyWorld::boot(v, &["Alice", "Bob"]).await;

        let fixture_dir = tempfile::tempdir().expect("fixture dir");
        // Single repeat — we want the song to end naturally.
        let wav = bm_wav(fixture_dir.path(), "track", 1);
        let (audio_id, duration_ms) = w
            .proc("Alice")
            .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(20))
            .expect("Alice uploads track");

        // Drive BOTH players far from the jukebox block so neither is in proximity.
        for _ in 0..5 {
            w.upstream.drive_position("Alice", FAR.0, FAR.1, FAR.2).await;
            w.upstream.drive_position("Bob", FAR.0 + 10.0, FAR.1, FAR.2).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // Insert the track with nobody in range — song starts and ends unheard.
        w.upstream
            .play_sound("Alice", &audio_id, JUKEBOX_BLOCK.0, JUKEBOX_BLOCK.1, JUKEBOX_BLOCK.2, "minecraft:overworld")
            .await;

        // Wait for the full duration + 1500 ms settle so the auto-eject fires.
        std::thread::sleep(Duration::from_millis(duration_ms as u64 + 1_500));

        // Drain Bob's buffer and snapshot his frame counter as the baseline.
        let _ = w.proc("Bob").drain_captured();
        let (_, b_fq_base, _) = w.proc("Bob").stats();

        // Now drive Bob INTO range — the song is already over.
        for _ in 0..5 {
            w.upstream.drive_position("Bob", NEAR.0, NEAR.1, NEAR.2).await;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // Capture ~2 s to give any stale frames time to materialise if the
        // server incorrectly replays or keeps the event alive.
        let cap_after = w.proc("Bob").collect_captured(Duration::from_millis(2_000));
        let (_, b_fq_after, _) = w.proc("Bob").stats();
        let rms_after = Signal::rms(&Signal::to_mono(&cap_after));

        eprintln!(
            "[proxy/C3c {v}] duration_ms={duration_ms} \
             b_fq_base={b_fq_base} b_fq_after={b_fq_after} rms_after={rms_after:.6}"
        );

        w.shutdown();

        assert_eq!(
            b_fq_after, b_fq_base,
            "[{v}] C3c: late arrival must receive zero incremental frames (song already ended)"
        );
        assert!(
            rms_after < 0.01,
            "[{v}] C3c: late arrival must be silent (rms={rms_after:.6})"
        );
    }
}
