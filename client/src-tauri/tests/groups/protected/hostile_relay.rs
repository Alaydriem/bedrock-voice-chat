//! A relay that attacks. Each case must end in a refused or failed check, different codes, or a
//! visible fork — never in a member holding keys to a group it was not truly admitted to.

use std::time::Duration;

use bvc_client_lib::groups::protected::{
    AdmissionRateLimiter, LostReason, MlsGroupState, ProtectedGroupEvent,
};
use bytes::Bytes;
use common::structs::group_key::{AdmissionMessage, GroupKeyFrame, GroupKeyPayload, SocketId};

use super::harness::{Hostility, TestGroup, TestRelay};

fn delivered(from: SocketId, message: AdmissionMessage) -> GroupKeyFrame {
    GroupKeyFrame::Delivered {
        from,
        body: GroupKeyPayload::Admission(message).to_bytes().unwrap(),
    }
}

fn lost_reason(group: &TestGroup, index: usize) -> Option<LostReason> {
    match group.members[index].lost() {
        Some(ProtectedGroupEvent::Lost(reason)) => Some(*reason),
        _ => None,
    }
}

fn check_failed(group: &TestGroup, member: usize, joiner: usize) -> bool {
    let socket = group.members[joiner].socket;
    group.members[member].has(|e| *e == ProtectedGroupEvent::CheckFailed { joiner: socket })
}

// The relay swaps the joiner's KeyPackage for its own after the commitment. The opening no
// longer matches the commitment, so the check fails before any code is shown.
#[test]
fn hostile_relay_swapping_the_key_package_fails_the_check() {
    let mut group = TestGroup::with_hostility(Hostility::SwapKeyPackage);
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();

    group.check(alice, bob);

    assert!(check_failed(&group, alice, bob));
    assert_eq!(group.member_code(alice, bob), None);
    assert_eq!(lost_reason(&group, bob), Some(LostReason::Refused));
    assert_eq!(group.roster(alice), vec!["minecraft:Alice"]);
}

// The relay swaps the member's nonce. Each side derives its code from what it saw, so the two
// codes differ and two people comparing them do not admit.
#[test]
fn hostile_relay_swapping_the_member_nonce_gives_different_codes() {
    let mut group = TestGroup::with_hostility(Hostility::SwapMemberNonce);
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();

    group.check(alice, bob);

    let (_, member_code) = group.member_code(alice, bob).expect("the member sees a code");
    let (_, joiner_code) = group.joiner_code(bob).expect("the joiner sees a code");
    assert_ne!(member_code, joiner_code);
}

// The relay swallows every reveal. Each check fails rather than quietly timing out, and after
// three the member can no longer check that identity.
#[test]
fn hostile_relay_aborting_reveals_raises_check_failed_and_hits_the_limit_after_three() {
    let mut group = TestGroup::with_hostility(Hostility::AbortReveals);
    let alice = group.create("Alice");

    for _ in 0..AdmissionRateLimiter::PER_JOINER {
        let bob = group.join_quietly("Bob");
        group.pump();
        group.check(alice, bob);
        group.advance(Duration::from_secs(61));
        assert!(check_failed(&group, alice, bob));
        group.disconnect(bob);
    }

    let bob = group.join_quietly("Bob");
    group.pump();
    let socket = group.members[bob].socket;
    assert!(group.members[alice].service.check(socket).is_err());
}

// "Someone else is checking" travels only inside the group's own encrypted messages. A bare one
// from the relay changes nothing on the member's screen.
#[test]
fn hostile_relay_forging_checking_does_not_close_prompts() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();
    let bob_socket = group.members[bob].socket;
    let alice_socket = group.members[alice].socket;

    group.relay.inject(
        alice_socket,
        delivered(TestRelay::IMPOSTOR, AdmissionMessage::Checking { joiner: bob_socket }),
    );
    group.pump();

    assert!(!group.members[alice].has(|e| matches!(e, ProtectedGroupEvent::Checking { .. })));
    assert!(group.members[alice].service.check(bob_socket).is_ok());
}

// The server sends the joiner a check of its own, then a Welcome into a group it runs, signed
// with the key from that check. The joiner never said the codes match, so it never enters.
#[test]
fn a_welcome_signed_by_the_check_key_without_confirmation_does_not_join() {
    let mut group = TestGroup::new();
    let _alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();
    let bob_socket = group.members[bob].socket;
    let mut server_group = MlsGroupState::create(&"minecraft:Server".parse().unwrap()).unwrap();
    let check_id = [0x77; 16];

    group.relay.inject(
        bob_socket,
        delivered(
            TestRelay::IMPOSTOR,
            AdmissionMessage::Check {
                check_id,
                nonce: [0x11; 32],
                signature_key: Bytes::from(server_group.own_signature_key()),
                mls_group_id: Bytes::from(server_group.mls_group_id()),
                member: "minecraft:Alice".to_string(),
            },
        ),
    );
    group.pump();
    let reveal_package = MlsGroupState::key_package(&"minecraft:Bob".parse().unwrap())
        .unwrap()
        .1;
    let staged = server_group.add(&reveal_package).unwrap();
    server_group.merge_pending().unwrap();
    group.relay.inject(
        bob_socket,
        delivered(
            TestRelay::IMPOSTOR,
            AdmissionMessage::Welcome {
                check_id,
                welcome: Bytes::from(staged.welcome),
            },
        ),
    );
    group.pump();
    group.advance(Duration::from_secs(5));

    assert!(!group.members[bob].joined());
    assert!(!group.members[bob].gate.has_keys());
}

// The relay already holds the joiner's opening from the first check. A second check, however
// late, gets `Busy` and no code: the server cannot try again against the same commitment.
#[test]
fn a_second_check_after_a_reveal_is_busy_even_after_expiry() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();
    group.check(alice, bob);
    let bob_socket = group.members[bob].socket;
    let first_code = group.joiner_code(bob).unwrap();

    group.relay.inject(
        bob_socket,
        delivered(
            TestRelay::IMPOSTOR,
            AdmissionMessage::Check {
                check_id: [0x42; 16],
                nonce: [0x24; 32],
                signature_key: Bytes::from_static(&[1; 32]),
                mls_group_id: Bytes::from_static(&[2; 16]),
                member: "minecraft:Server".to_string(),
            },
        ),
    );
    group.pump();

    assert_eq!(group.joiner_code(bob), Some(first_code), "a second code was derived");
}

#[test]
fn relay_replaying_an_old_commit_is_rejected() {
    let mut group = TestGroup::with_hostility(Hostility::ReplayFirstCommit);
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join("Carol");

    for member in [alice, bob, carol] {
        assert_eq!(lost_reason(&group, member), None, "member {member}");
    }
    assert_eq!(group.epoch(alice), group.epoch(carol));
    assert_eq!(group.safety_code(alice), group.safety_code(bob));
    assert_eq!(group.safety_code(bob), group.safety_code(carol));
}

// Anyone on the server can open a key socket on a protected channel. A commit from such a socket
// is not a member's, so it must change nothing — not end the group, not stall its next commit.
#[test]
fn a_garbage_commit_from_a_stranger_is_ignored() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let epoch = group.epoch(alice).unwrap();

    group.relay.inject_all(GroupKeyFrame::Commit {
        epoch,
        body: Bytes::from_static(b"not a commit from any member"),
    });
    group.pump();

    for member in [alice, bob] {
        assert_eq!(lost_reason(&group, member), None, "member {member} lost the group");
        assert_eq!(group.epoch(member), Some(epoch));
    }
    group.advance(Duration::from_secs(5 * 60 + 1));
    assert_eq!(group.epoch(alice), Some(epoch + 1), "the group can no longer ratchet");
    assert_eq!(group.safety_code(alice), group.safety_code(bob));
}

// A stranger cannot end someone else's join by sending it a refusal.
#[test]
fn a_refused_from_a_stranger_does_not_end_a_join() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();
    group.check(alice, bob);
    let (check_id, _) = group.joiner_code(bob).unwrap();
    let bob_socket = group.members[bob].socket;

    group.relay.inject(
        bob_socket,
        delivered(TestRelay::IMPOSTOR, AdmissionMessage::Refused { check_id }),
    );
    group.pump();
    let (member_check, _) = group.member_code(alice, bob).unwrap();
    group.decide(alice, bob, member_check, true);
    group.confirm(bob, check_id, true);

    assert_eq!(lost_reason(&group, bob), None);
    assert!(group.members[bob].joined());
    assert_eq!(group.roster(alice).len(), 2);
}

#[test]
fn relay_forking_the_group_shows_different_safety_codes() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    assert_eq!(group.safety_code(alice), group.safety_code(bob));

    group.relay.hostility = Hostility::ForkOnNextCommit;
    group.advance(Duration::from_secs(5 * 60 + 4));

    assert_eq!(group.epoch(alice), group.epoch(bob));
    assert_ne!(group.safety_code(alice), group.safety_code(bob));
}
