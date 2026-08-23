use std::time::{Duration, Instant};

use bvc_client_lib::audio::{LevelEmitPolicy, LoudnessTracker};
use common::structs::audio::{LevelSnapshot, ParticipantLevel};

fn speaking(loudness: u8) -> ParticipantLevel {
    ParticipantLevel {
        speaking: true,
        loudness,
    }
}

fn own(level: ParticipantLevel) -> LevelSnapshot {
    LevelSnapshot {
        own: level,
        peers: Default::default(),
    }
}

fn peer(name: &str, level: ParticipantLevel) -> LevelSnapshot {
    let mut snapshot = LevelSnapshot::silent();
    snapshot.peers.insert(name.to_string(), level);
    snapshot
}

mod loudness {
    use super::*;
    use bvc_client_lib::audio::LevelEmitPolicy;

    /// Ordinary speech is speech. The floor sits at a quiet room, so anything a person actually
    /// says clears it comfortably — and when the noise gate is on, its own threshold is well
    /// above this floor, so audio it opened for can never land back on step zero.
    #[test]
    fn ordinary_speech_reports_as_speaking() {
        let mut tracker = LoudnessTracker::new();

        // 0.02 to 0.08 RMS is speech into a desk microphone.
        for rms in [0.02, 0.05, 0.08] {
            let level = tracker.observe(rms, true);
            assert!(level.speaking, "{rms} should read as speech");
            assert!(level.loudness >= 1);
        }
    }

    /// The bug this pair exists for. `passing` alone was the answer, and with the noise gate off
    /// it is derived from whether any sample is exactly zero — which a live microphone never
    /// satisfies. So a speaker's own level said "speaking" forever and never changed, while
    /// everyone else's moved; a state that never changes is never worth a message, so their
    /// meter went still and stayed still.
    #[test]
    fn room_noise_that_the_path_let_through_is_not_speech() {
        let mut tracker = LoudnessTracker::new();

        // Far below the meter's floor, but non-zero — exactly what an open path carries between
        // words when nothing is gating it.
        let level = tracker.observe(0.000_01, true);
        assert!(!level.speaking);
        assert_eq!(level.loudness, 0);
    }

    #[test]
    fn a_voice_over_room_noise_transitions_rather_than_reading_as_speech_throughout() {
        let mut tracker = LoudnessTracker::new();

        // Nothing here is ever gated: `passing` stays true the whole time, the way it does with
        // the noise gate switched off.
        assert!(!tracker.observe(0.000_02, true).speaking);
        assert!(tracker.observe(0.05, true).speaking);
        assert!(!tracker.observe(0.000_02, true).speaking);
    }

    /// The end-to-end shape of the same defect, at the layer that showed it. A speaker whose
    /// audio path never gates has to produce messages while they talk, or their meter is driven
    /// by the keepalive alone and reads as frozen next to everybody else's.
    #[test]
    fn a_speaker_on_an_ungated_path_still_produces_messages_while_talking() {
        let mut tracker = LoudnessTracker::new();
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        let mut sent = 0;

        // Two seconds of alternating speech and pauses over an open path, sampled the way the
        // publisher samples.
        for tick in 0..20u64 {
            let rms = if (tick / 3) % 2 == 0 { 0.05 } else { 0.000_02 };
            let level = tracker.observe(rms, true);
            if policy.admit(base + Duration::from_millis(tick * 100), &own(level)) {
                sent += 1;
            }
        }

        // Keepalives alone would be two over two seconds. Anything more is the transitions this
        // was missing, and those are what make the meter move.
        assert!(
            sent > 2,
            "a talking speaker must produce more than the keepalive, sent {sent}"
        );
    }

    #[test]
    fn audio_the_gate_stopped_is_silence_however_loud_it_measured() {
        let mut tracker = LoudnessTracker::new();
        tracker.observe(0.05, true);

        let level = tracker.observe(0.5, false);
        assert!(!level.speaking);
        assert_eq!(level.loudness, 0);
    }

    #[test]
    fn louder_audio_reports_a_higher_step() {
        let mut quiet = LoudnessTracker::new();
        let mut loud = LoudnessTracker::new();

        assert!(loud.observe(0.2, true).loudness > quiet.observe(0.005, true).loudness);
    }

    /// The reason for quantising at all. A step that changes is what buys a webview message, so
    /// a voice sitting between two steps must not flip between them — otherwise a steady
    /// speaker costs as much traffic as a changing one, and the quantisation has bought nothing.
    #[test]
    fn a_level_resting_on_a_boundary_holds_its_step() {
        let mut tracker = LoudnessTracker::new();
        let settled = tracker.observe(0.02, true).loudness;

        // Small wobbles either side of wherever that landed.
        let mut steps = Vec::new();
        for rms in [0.0205, 0.0195, 0.0202, 0.0198, 0.0201] {
            steps.push(tracker.observe(rms, true).loudness);
        }

        assert!(
            steps.iter().all(|s| *s == settled),
            "expected the step to hold at {settled}, got {steps:?}"
        );
    }

    #[test]
    fn a_real_change_in_level_does_move_the_step() {
        let mut tracker = LoudnessTracker::new();
        let quiet = tracker.observe(0.004, true).loudness;
        let loud = tracker.observe(0.3, true).loudness;

        // Hysteresis holds a step against noise. It must not hold it against somebody actually
        // raising their voice, or the meter is a light rather than a meter.
        assert!(loud > quiet, "{quiet} -> {loud}");
    }
}

mod policy {
    use super::*;

    fn at(base: Instant, millis: u64) -> Instant {
        base + Duration::from_millis(millis)
    }

    /// Silence is what a client already draws at rest, so saying so costs a message and buys
    /// nothing. This is most of the traffic in a normal session: a room where nobody is talking
    /// used to cost twenty messages a second.
    #[test]
    fn silence_is_never_worth_a_message() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();

        for tick in 0..50 {
            assert!(
                !policy.admit(at(base, tick * 100), &LevelSnapshot::silent()),
                "tick {tick}"
            );
        }
    }

    /// The transition the user is watching for. Rate-limiting it would make a press-to-talk read
    /// as a control that did not take, which is the whole reason the meter is on screen.
    #[test]
    fn somebody_starting_to_speak_is_sent_immediately() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();

        assert!(policy.admit(base, &own(speaking(4))));
    }

    #[test]
    fn somebody_stopping_is_sent_immediately() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        policy.admit(base, &own(speaking(4)));

        // Straight after the last message, well inside every rate limit.
        assert!(policy.admit(at(base, 10), &LevelSnapshot::silent()));
    }

    #[test]
    fn a_peer_starting_is_sent_immediately_even_while_others_speak() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        policy.admit(base, &own(speaking(4)));

        let mut both = own(speaking(4));
        both.peers.insert("Petra".to_string(), speaking(3));
        assert!(policy.admit(at(base, 20), &both));
    }

    /// A peer who leaves stops appearing at all, and nothing else in the payload says they
    /// stopped. Their meter would hold its last level until the client's own decay caught up.
    #[test]
    fn a_peer_disappearing_counts_as_stopping() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        policy.admit(base, &peer("Petra", speaking(5)));

        assert!(policy.admit(at(base, 20), &LevelSnapshot::silent()));
    }

    /// Speech changes at syllable rate. Sending every change is several messages a second for
    /// motion the client carries itself — the meter eases between heights and animates in
    /// between — which is what this whole design is avoiding.
    ///
    /// Measured against the constant rather than a hardcoded number of ticks. Written the other
    /// way it passed until the gap was tuned, then failed on the tuning rather than on a
    /// regression, which is the opposite of what a test is for.
    #[test]
    fn amplitude_alone_does_not_buy_a_message_before_the_minimum_gap() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        policy.admit(base, &own(speaking(4)));

        let gap = LevelEmitPolicy::MIN_GAP.as_millis() as u64;
        for elapsed in [1, gap / 2, gap - 1] {
            assert!(
                !policy.admit(at(base, elapsed), &own(speaking(7))),
                "{elapsed} ms in, only the amplitude had changed"
            );
        }
    }

    #[test]
    fn amplitude_is_sent_once_the_minimum_gap_has_passed() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        policy.admit(base, &own(speaking(4)));

        let gap = LevelEmitPolicy::MIN_GAP.as_millis() as u64;
        assert!(policy.admit(at(base, gap), &own(speaking(7))));
    }

    #[test]
    fn an_unchanged_level_is_not_resent_merely_because_the_gap_passed() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        policy.admit(base, &own(speaking(4)));

        let gap = LevelEmitPolicy::MIN_GAP.as_millis() as u64;
        assert!(!policy.admit(at(base, gap), &own(speaking(4))));
    }

    /// The client expires a speaking flag it has not heard about, so a meter cannot animate
    /// forever over a backend that died. This is what stops that expiry firing mid-sentence.
    #[test]
    fn a_steady_voice_is_refreshed_before_the_client_would_give_up_on_it() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        policy.admit(base, &own(speaking(4)));

        let keepalive = LevelEmitPolicy::KEEPALIVE.as_millis() as u64;
        assert!(policy.admit(at(base, keepalive), &own(speaking(4))));
    }

    /// The number the design is judged on. A full minute of one person talking steadily, sampled
    /// ten times a second the way the publisher does, must not approach what the two fixed-rate
    /// emitters cost — which was 1,200 messages over the same minute.
    #[test]
    fn a_minute_of_steady_speech_costs_a_few_messages_a_second_at_most() {
        let mut policy = LevelEmitPolicy::new();
        let base = Instant::now();
        let mut sent = 0;

        for tick in 0..600u64 {
            // A voice wandering across two steps, which is what the tracker's hysteresis leaves
            // after a real speaker.
            let loudness = if (tick / 7) % 2 == 0 { 4 } else { 5 };
            if policy.admit(at(base, tick * 100), &own(speaking(loudness))) {
                sent += 1;
            }
        }

        assert!(
            sent <= 180,
            "expected at most three a second over a minute, sent {sent}"
        );
        // And not zero: a client that hears nothing for a minute expires the speaking flag and
        // stops the meter over somebody who never stopped talking.
        assert!(sent >= 60, "a steady voice must still be refreshed, sent {sent}");
    }
}
