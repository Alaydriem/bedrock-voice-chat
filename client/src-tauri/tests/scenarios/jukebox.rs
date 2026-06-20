use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::jukebox_fixture::{A_NOTES, BM_NOTES, JukeboxFixture};
use crate::harness::jukebox_world::JukeboxWorld;
use crate::harness::note_energy::NoteEnergy;

// Within proximity (server broadcast_range default 48 → gate ≈ 1.73*48 ≈ 83 blocks).
const NEAR: (f32, f32, f32) = (0.0, 64.0, 0.0);
// Far beyond every proximity gate.
const FAR: (f32, f32, f32) = (10_000.0, 64.0, 0.0);

/// Case 1: two players both within proximity of one jukebox both hear the song.
#[tokio::test(flavor = "multi_thread")]
async fn both_in_range_hear_jukebox() {
    let w = JukeboxWorld::boot().await;

    // Alice uploads the Bm progression through the real client encode+upload path.
    let fixture_dir = tempfile::tempdir().expect("fixture dir");
    let wav = JukeboxFixture::bm_wav(fixture_dir.path(), "bm", 1);
    let (audio_id, duration_ms) = w
        .alice
        .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(20))
        .expect("Alice uploads the Bm fixture");
    assert!(duration_ms > 0, "uploaded fixture has non-zero duration");

    // Both players stand at the jukebox position; the jukebox plays there.
    for _ in 0..5 {
        w.server.update_positions(&[
            ("Alice", NEAR.0, NEAR.1, NEAR.2),
            ("Bob", NEAR.0, NEAR.1, NEAR.2),
        ]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));

    let (_event_id, _) = w
        .server
        .jukebox_play(&audio_id, NEAR.0, NEAR.1, NEAR.2)
        .await;

    // Capture across the playback window (duration + drain headroom). Both clients'
    // capture buffers accumulate continuously; draining Alice's long window first
    // does not starve Bob's (collect_captured takes whatever has accumulated).
    let window = Duration::from_millis(duration_ms as u64 + 1_500);
    let cap_a = w.alice.collect_captured(window);
    let cap_b = w.bob.collect_captured(Duration::from_millis(200));

    let (_, a_fq, _) = w.alice.stats();
    let (_, b_fq, _) = w.bob.stats();
    let mono_a = Signal::to_mono(&cap_a);
    let mono_b = Signal::to_mono(&cap_b);

    eprintln!(
        "[jukebox/case1] a_fq={a_fq} b_fq={b_fq} rms_a={:.4} rms_b={:.4}",
        Signal::rms(&mono_a),
        Signal::rms(&mono_b)
    );

    w.alice.shutdown();
    w.bob.shutdown();

    assert!(
        a_fq > 0 && b_fq > 0,
        "both clients must receive jukebox frames from QUIC"
    );
    assert!(
        NoteEnergy::all_present(&mono_a, &BM_NOTES),
        "Alice must hear every Bm note"
    );
    assert!(
        NoteEnergy::all_present(&mono_b, &BM_NOTES),
        "Bob must hear every Bm note"
    );
}

/// Case 2: jukebox at the origin; Alice in range hears it, Bob far away does not.
#[tokio::test(flavor = "multi_thread")]
async fn in_range_hears_out_of_range_silent_jukebox() {
    let w = JukeboxWorld::boot().await;

    let fixture_dir = tempfile::tempdir().expect("fixture dir");
    let wav = JukeboxFixture::bm_wav(fixture_dir.path(), "bm", 1);
    let (audio_id, duration_ms) = w
        .alice
        .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(20))
        .expect("upload Bm fixture");

    for _ in 0..5 {
        w.server.update_positions(&[
            ("Alice", NEAR.0, NEAR.1, NEAR.2),
            ("Bob", FAR.0, FAR.1, FAR.2),
        ]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(600));

    w.server
        .jukebox_play(&audio_id, NEAR.0, NEAR.1, NEAR.2)
        .await;

    let window = Duration::from_millis(duration_ms as u64 + 1_500);
    let cap_a = w.alice.collect_captured(window);
    let cap_b = w.bob.collect_captured(Duration::from_millis(200));

    let (_, a_fq, _) = w.alice.stats();
    let (_, b_fq, _) = w.bob.stats();
    let mono_a = Signal::to_mono(&cap_a);
    let rms_b = Signal::rms(&Signal::to_mono(&cap_b));
    eprintln!("[jukebox/case2] a_fq={a_fq} b_fq={b_fq} rms_b={rms_b:.6}");

    w.alice.shutdown();
    w.bob.shutdown();

    assert!(
        a_fq > 0 && NoteEnergy::all_present(&mono_a, &BM_NOTES),
        "Alice in range must hear the jukebox"
    );
    assert_eq!(
        b_fq, 0,
        "Bob out of range must receive zero jukebox frames from QUIC"
    );
    assert!(
        rms_b < 0.01,
        "Bob out of range must be silent (rms={rms_b:.6})"
    );
}

/// Case 3: jukebox at the origin; both players far away → neither hears it.
#[tokio::test(flavor = "multi_thread")]
async fn both_out_of_range_silent_jukebox() {
    let w = JukeboxWorld::boot().await;

    let fixture_dir = tempfile::tempdir().expect("fixture dir");
    let wav = JukeboxFixture::bm_wav(fixture_dir.path(), "bm", 1);
    let (audio_id, duration_ms) = w
        .alice
        .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(20))
        .expect("upload Bm fixture");

    for _ in 0..5 {
        w.server.update_positions(&[
            ("Alice", FAR.0, FAR.1, FAR.2),
            ("Bob", FAR.0 + 1.0, FAR.1, FAR.2),
        ]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(600));

    // Jukebox at the origin — far from both players.
    w.server
        .jukebox_play(&audio_id, NEAR.0, NEAR.1, NEAR.2)
        .await;

    let window = Duration::from_millis(duration_ms as u64 + 1_000);
    let cap_a = w.alice.collect_captured(window);
    let cap_b = w.bob.collect_captured(Duration::from_millis(200));

    let (_, a_fq, _) = w.alice.stats();
    let (_, b_fq, _) = w.bob.stats();
    let rms_a = Signal::rms(&Signal::to_mono(&cap_a));
    let rms_b = Signal::rms(&Signal::to_mono(&cap_b));
    eprintln!("[jukebox/case3] a_fq={a_fq} b_fq={b_fq} rms_a={rms_a:.6} rms_b={rms_b:.6}");

    w.alice.shutdown();
    w.bob.shutdown();

    assert_eq!(
        a_fq, 0,
        "Alice out of range must receive zero jukebox frames"
    );
    assert_eq!(b_fq, 0, "Bob out of range must receive zero jukebox frames");
    assert!(
        rms_a < 0.01 && rms_b < 0.01,
        "both must be silent (rms_a={rms_a:.6} rms_b={rms_b:.6})"
    );
}

/// Case 4: a 10x Bm loop is ejected after ~2 loops; the remaining loops never
/// play on either client. Proves DELETE /api/audio/event/{id} truncates playback.
#[tokio::test(flavor = "multi_thread")]
async fn eject_truncates_remaining_loops() {
    let w = JukeboxWorld::boot().await;

    let fixture_dir = tempfile::tempdir().expect("fixture dir");
    let wav = JukeboxFixture::bm_wav(fixture_dir.path(), "bm10", 10);
    let (audio_id, duration_ms) = w
        .alice
        .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(30))
        .expect("upload Bm x10 fixture");
    let per_loop_ms = duration_ms / 10;

    for _ in 0..5 {
        w.server.update_positions(&[
            ("Alice", NEAR.0, NEAR.1, NEAR.2),
            ("Bob", NEAR.0, NEAR.1, NEAR.2),
        ]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));

    let (event_id, _) = w
        .server
        .jukebox_play(&audio_id, NEAR.0, NEAR.1, NEAR.2)
        .await;

    // Let ~2 loops play, then eject.
    std::thread::sleep(Duration::from_millis((per_loop_ms * 2) as u64));
    w.server.jukebox_stop(&event_id).await;

    let (_, a_fq_at_eject, _) = w.alice.stats();
    let (_, b_fq_at_eject, _) = w.bob.stats();

    // Give a generous post-eject window; if eject worked, frames stop almost
    // immediately, so this delta stays tiny rather than accumulating ~8 more loops.
    std::thread::sleep(Duration::from_millis(3_000));

    let (_, a_fq_final, _) = w.alice.stats();
    let (_, b_fq_final, _) = w.bob.stats();
    eprintln!(
        "[jukebox/case4] per_loop_ms={per_loop_ms} a@eject={a_fq_at_eject} a_final={a_fq_final} \
         b@eject={b_fq_at_eject} b_final={b_fq_final}"
    );

    w.alice.shutdown();
    w.bob.shutdown();

    assert!(
        a_fq_at_eject > 0 && b_fq_at_eject > 0,
        "both should hear the first loops"
    );

    let full_frames = (duration_ms as u64) / 20;
    let one_loop_frames = (per_loop_ms as u64) / 20;
    let a_delta = (a_fq_final as u64).saturating_sub(a_fq_at_eject as u64);
    let b_delta = (b_fq_final as u64).saturating_sub(b_fq_at_eject as u64);
    assert!(
        a_delta < one_loop_frames && b_delta < one_loop_frames,
        "post-eject frames ({a_delta}/{b_delta}) must be < one loop ({one_loop_frames}); eject did not truncate"
    );
    assert!(
        (a_fq_final as u64) < full_frames / 2 && (b_fq_final as u64) < full_frames / 2,
        "total frames must be well below a full 10x playback ({full_frames})"
    );
}

/// Case 5: a single progression plays fully, then capture returns to silence and
/// no further frames arrive (the playback task ends + auto-eject fires).
#[tokio::test(flavor = "multi_thread")]
async fn natural_end_returns_to_silence() {
    let w = JukeboxWorld::boot().await;

    let fixture_dir = tempfile::tempdir().expect("fixture dir");
    let wav = JukeboxFixture::bm_wav(fixture_dir.path(), "bm", 1);
    let (audio_id, duration_ms) = w
        .alice
        .upload_audio(wav.to_str().unwrap(), "minecraft", Duration::from_secs(20))
        .expect("upload Bm fixture");

    for _ in 0..5 {
        w.server.update_positions(&[
            ("Alice", NEAR.0, NEAR.1, NEAR.2),
            ("Bob", NEAR.0, NEAR.1, NEAR.2),
        ]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));

    w.server
        .jukebox_play(&audio_id, NEAR.0, NEAR.1, NEAR.2)
        .await;

    // Phase A: capture the full playback → energy present.
    let cap_play = w
        .alice
        .collect_captured(Duration::from_millis(duration_ms as u64 + 1_000));
    assert!(
        NoteEnergy::all_present(&Signal::to_mono(&cap_play), &BM_NOTES),
        "Alice must hear the progression during playback"
    );

    // Let the playback fully finish + drain, then baseline the frame counter.
    std::thread::sleep(Duration::from_millis(1_500));
    let (_, a_fq_baseline, _) = w.alice.stats();

    // Phase B: a fresh capture window AFTER the song ended → silence, no new frames.
    let cap_after = w.alice.collect_captured(Duration::from_millis(2_000));
    let (_, a_fq_after, _) = w.alice.stats();
    let rms_after = Signal::rms(&Signal::to_mono(&cap_after));
    eprintln!(
        "[jukebox/case5] a_fq_baseline={a_fq_baseline} a_fq_after={a_fq_after} rms_after={rms_after:.6}"
    );

    w.alice.shutdown();
    w.bob.shutdown();

    assert_eq!(
        a_fq_after, a_fq_baseline,
        "no jukebox frames must arrive after the song ended"
    );
    assert!(
        rms_after < 0.01,
        "capture must be silent after the song ended (rms={rms_after:.6})"
    );
}

/// Case 6: two jukeboxes at two distant positions play different progressions
/// (Bm vs high A-major). Alice (at A) hears Bm only; Bob (at B) hears A-major only.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_jukeboxes_no_cross_bleed() {
    let w = JukeboxWorld::boot().await;

    let fixture_dir = tempfile::tempdir().expect("fixture dir");
    let bm = JukeboxFixture::bm_wav(fixture_dir.path(), "bm", 2);
    let amaj = JukeboxFixture::a_major_wav(fixture_dir.path(), "amaj", 2);
    let (bm_id, bm_dur) = w
        .alice
        .upload_audio(bm.to_str().unwrap(), "minecraft", Duration::from_secs(20))
        .expect("upload Bm");
    let (amaj_id, amaj_dur) = w
        .alice
        .upload_audio(amaj.to_str().unwrap(), "minecraft", Duration::from_secs(20))
        .expect("upload A-major");

    let site_a = (0.0_f32, 64.0, 0.0);
    let site_b = (10_000.0_f32, 64.0, 0.0);
    for _ in 0..5 {
        w.server.update_positions(&[
            ("Alice", site_a.0, site_a.1, site_a.2),
            ("Bob", site_b.0, site_b.1, site_b.2),
        ]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));

    // Distinct coords → no dedup collision.
    w.server
        .jukebox_play(&bm_id, site_a.0, site_a.1, site_a.2)
        .await;
    w.server
        .jukebox_play(&amaj_id, site_b.0, site_b.1, site_b.2)
        .await;

    let window = Duration::from_millis(bm_dur.max(amaj_dur) as u64 + 1_500);
    let cap_a = w.alice.collect_captured(window);
    let cap_b = w.bob.collect_captured(Duration::from_millis(200));

    let (_, a_fq, _) = w.alice.stats();
    let (_, b_fq, _) = w.bob.stats();
    let mono_a = Signal::to_mono(&cap_a);
    let mono_b = Signal::to_mono(&cap_b);
    eprintln!("[jukebox/case6] a_fq={a_fq} b_fq={b_fq}");

    w.alice.shutdown();
    w.bob.shutdown();

    assert!(
        a_fq > 0 && b_fq > 0,
        "both clients must receive their own jukebox frames"
    );
    assert!(
        NoteEnergy::all_present(&mono_a, &BM_NOTES),
        "Alice must hear the Bm progression"
    );
    assert!(
        NoteEnergy::all_absent(&mono_a, &A_NOTES),
        "Alice must NOT hear the A-major progression"
    );
    assert!(
        NoteEnergy::all_present(&mono_b, &A_NOTES),
        "Bob must hear the A-major progression"
    );
    assert!(
        NoteEnergy::all_absent(&mono_b, &BM_NOTES),
        "Bob must NOT hear the Bm progression"
    );
}
