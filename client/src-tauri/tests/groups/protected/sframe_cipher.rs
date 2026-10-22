use std::time::{Duration, Instant};

use bvc_client_lib::groups::protected::SframeCipher;
use common::errors::SframeError;

const EPOCH: u64 = 7;

fn base_keys() -> Vec<(u32, [u8; 32])> {
    vec![(0, [0x11; 32]), (1, [0x22; 32]), (2, [0x33; 32])]
}

fn pair(now: Instant) -> (SframeCipher, SframeCipher) {
    let mut alice = SframeCipher::new(0);
    let mut bob = SframeCipher::new(1);
    alice.install_epoch(EPOCH, base_keys(), now);
    bob.install_epoch(EPOCH, base_keys(), now);
    (alice, bob)
}

const OPUS: &[u8] = &[0xAB; 80];

#[test]
fn sealed_frame_opens_for_another_member_and_names_the_sender() {
    let now = Instant::now();
    let (mut alice, mut bob) = pair(now);

    let sealed = alice.seal(OPUS).unwrap();
    let opened = bob.open(&sealed, now).unwrap();

    assert_eq!(opened.sender_leaf, 0);
    assert_eq!(opened.opus, OPUS);
}

// The per-frame cost the bandwidth figure in the spec rests on: header plus a 16-byte tag.
#[test]
fn sealed_frame_differs_from_plaintext_and_carries_bounded_overhead() {
    let now = Instant::now();
    let (mut alice, _bob) = pair(now);

    let first = alice.seal(OPUS).unwrap();
    for _ in 0..999 {
        alice.seal(OPUS).unwrap();
    }
    let thousandth = alice.seal(OPUS).unwrap();

    assert!(!first.windows(OPUS.len()).any(|w| w == OPUS));
    for sealed in [&first, &thousandth] {
        let overhead = sealed.len() - OPUS.len();
        assert!((17..=24).contains(&overhead), "overhead {overhead} bytes");
    }
}

#[test]
fn receiver_steps_forward_across_a_ratchet_boundary_after_loss() {
    let now = Instant::now();
    let (mut alice, mut bob) = pair(now);

    let frames: Vec<Vec<u8>> = (0..200).map(|_| alice.seal(OPUS).unwrap()).collect();

    bob.open(&frames[0], now).unwrap();
    // Frames 60..=130 are lost, which spans the step boundaries at 64 and 128.
    let opened = bob.open(&frames[131], now).unwrap();
    assert_eq!(opened.opus, OPUS);
}

// Forward secrecy inside an epoch: once a receiver has stepped past a key it no longer holds it.
#[test]
fn receiver_cannot_open_a_frame_from_an_earlier_step_after_stepping() {
    let now = Instant::now();
    let (mut alice, mut bob) = pair(now);

    let frames: Vec<Vec<u8>> = (0..130).map(|_| alice.seal(OPUS).unwrap()).collect();
    bob.open(&frames[129], now).unwrap();

    assert!(bob.open(&frames[3], now).is_err());
}

#[test]
fn replayed_frame_is_rejected() {
    let now = Instant::now();
    let (mut alice, mut bob) = pair(now);

    let sealed = alice.seal(OPUS).unwrap();
    bob.open(&sealed, now).unwrap();

    assert_eq!(bob.open(&sealed, now).unwrap_err(), SframeError::Replay);
}

#[test]
fn previous_epoch_opens_within_2s_and_not_after() {
    let now = Instant::now();
    let (mut alice, mut bob) = pair(now);
    let in_flight = alice.seal(OPUS).unwrap();
    let later_flight = alice.seal(OPUS).unwrap();

    let next_keys = vec![(0, [0x44; 32]), (1, [0x55; 32]), (2, [0x66; 32])];
    bob.install_epoch(EPOCH + 1, next_keys, now);

    assert!(bob.open(&in_flight, now + Duration::from_millis(1_900)).is_ok());
    bob.expire(now + Duration::from_millis(2_100));
    assert!(bob.open(&later_flight, now + Duration::from_millis(2_100)).is_err());
}

#[test]
fn tampered_ciphertext_fails_authentication() {
    let now = Instant::now();
    let (mut alice, mut bob) = pair(now);

    let mut sealed = alice.seal(OPUS).unwrap();
    let last = sealed.len() - 1;
    sealed[last] ^= 0x01;

    assert_eq!(bob.open(&sealed, now).unwrap_err(), SframeError::Authentication);
}

#[test]
fn seal_without_epoch_is_no_epoch() {
    let mut alice = SframeCipher::new(0);

    assert_eq!(alice.seal(OPUS).unwrap_err(), SframeError::NoEpoch);
}

#[test]
fn a_frame_from_a_leaf_outside_the_epoch_is_unknown_key() {
    let now = Instant::now();
    let mut stranger = SframeCipher::new(9);
    stranger.install_epoch(EPOCH, vec![(9, [0x99; 32])], now);
    let (_alice, mut bob) = pair(now);

    let sealed = stranger.seal(OPUS).unwrap();

    assert_eq!(bob.open(&sealed, now).unwrap_err(), SframeError::UnknownKey);
}
