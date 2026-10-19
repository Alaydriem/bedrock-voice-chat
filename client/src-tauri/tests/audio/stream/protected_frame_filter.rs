use std::sync::Arc;
use std::time::Instant;

use bvc_client_lib::audio::stream::protected_frame_filter::{OutboundDecision, ProtectedFrameFilter};
use bvc_client_lib::groups::protected::{MlsGroupState, SealGate};
use common::structs::packet::AudioFramePacket;

const OPUS: &[u8] = &[0x33; 80];

/// Alice's and Bob's gates, holding one epoch of a real two-member group.
fn gates() -> (Arc<SealGate>, Arc<SealGate>) {
    let mut alice = MlsGroupState::create(&"minecraft:Alice".parse().unwrap()).unwrap();
    let (pending, key_package) = MlsGroupState::key_package(&"minecraft:Bob".parse().unwrap()).unwrap();
    let staged = alice.add(&key_package).unwrap();
    alice.merge_pending().unwrap();
    let bob = pending.join(&staged.welcome).unwrap();

    let now = Instant::now();
    let install = |state: &MlsGroupState| {
        let gate = Arc::new(SealGate::new());
        gate.set_protected(true);
        let session = gate.begin_session();
        gate.install_epoch(session, state.own_leaf(), state.epoch(), state.sframe_base_keys(), &state.roster(), now);
        gate
    };
    (install(&alice), install(&bob))
}

// The one rule that must never break: a protected member with no keys sends nothing, rather than
// falling back to plaintext the server and nearby players could hear.
#[test]
fn outbound_when_protected_without_keys_sends_nothing() {
    let gate = Arc::new(SealGate::new());
    gate.set_protected(true);
    let filter = ProtectedFrameFilter::new(gate);

    assert_eq!(filter.outbound(OPUS), OutboundDecision::Drop);
}

#[test]
fn outbound_when_protected_sends_sealed_never_plaintext() {
    let (alice, _bob) = gates();
    let filter = ProtectedFrameFilter::new(alice);

    match filter.outbound(OPUS) {
        OutboundDecision::Sealed(sealed) => {
            assert!(!sealed.windows(OPUS.len()).any(|w| w == OPUS));
        }
        other => panic!("expected a sealed frame, got {other:?}"),
    }
}

#[test]
fn outbound_when_unprotected_passes_through_unchanged() {
    let filter = ProtectedFrameFilter::new(Arc::new(SealGate::new()));

    assert_eq!(filter.outbound(OPUS), OutboundDecision::Plain(OPUS.to_vec()));
}

#[test]
fn inbound_sealed_from_the_member_whose_key_sealed_it_opens() {
    let (alice, bob) = gates();
    let OutboundDecision::Sealed(sealed) = ProtectedFrameFilter::new(alice).outbound(OPUS) else {
        panic!("alice holds keys");
    };
    let frame = AudioFramePacket::new(OPUS.to_vec(), None, None).into_sealed(sealed);

    let opened = ProtectedFrameFilter::new(bob).inbound(&frame, "minecraft:Alice");

    assert_eq!(opened.as_deref(), Some(OPUS));
}

// The server stamps the sender; the key says who really sealed it. A mismatch is dropped.
#[test]
fn inbound_sealed_with_mismatched_sender_is_dropped() {
    let (alice, bob) = gates();
    let OutboundDecision::Sealed(sealed) = ProtectedFrameFilter::new(alice).outbound(OPUS) else {
        panic!("alice holds keys");
    };
    let frame = AudioFramePacket::new(OPUS.to_vec(), None, None).into_sealed(sealed);

    assert!(ProtectedFrameFilter::new(bob).inbound(&frame, "minecraft:Mallory").is_none());
}

// A protected member still hears nearby players, whose audio is ordinary plaintext.
#[test]
fn inbound_unsealed_proximity_audio_passes_while_protected() {
    let (_alice, bob) = gates();
    let frame = AudioFramePacket::new(OPUS.to_vec(), None, Some(true));

    assert_eq!(
        ProtectedFrameFilter::new(bob).inbound(&frame, "minecraft:Carol").as_deref(),
        Some(OPUS)
    );
}

// A group member only ever sends sealed frames, so a plain frame stamped with a member's name was
// made up — by the server, which stamps senders. Playing it would put words in that member's mouth.
#[test]
fn inbound_unsealed_from_a_group_member_while_protected_is_dropped() {
    let (_alice, bob) = gates();
    let forged = AudioFramePacket::new(OPUS.to_vec(), None, Some(false));

    assert!(ProtectedFrameFilter::new(bob).inbound(&forged, "minecraft:Alice").is_none());
}

// The gate belongs to the group, not to the voice link: a reconnected link keeps sealing with
// the same keys rather than dropping to nothing or to plaintext.
#[test]
fn sealing_survives_voice_link_reconnect() {
    let (alice, bob) = gates();
    let before = ProtectedFrameFilter::new(alice.clone());
    assert!(matches!(before.outbound(OPUS), OutboundDecision::Sealed(_)));
    drop(before);

    let after_reconnect = ProtectedFrameFilter::new(alice);
    let OutboundDecision::Sealed(sealed) = after_reconnect.outbound(OPUS) else {
        panic!("still sealing after the link was rebuilt");
    };
    let frame = AudioFramePacket::new(OPUS.to_vec(), None, None).into_sealed(sealed);
    assert!(ProtectedFrameFilter::new(bob).inbound(&frame, "minecraft:Alice").is_some());
}
