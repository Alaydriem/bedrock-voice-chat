//! A relay that attacks. Each case must end in a refusal or a visible fork, never in a member
//! holding keys to a group it was not truly admitted to.

use std::time::Duration;

use bvc_client_lib::groups::protected::{LostReason, ProtectedGroupEvent};
use bytes::Bytes;
use common::structs::group_key::{AdmissionMessage, GroupKeyFrame, GroupKeyPayload};

use super::harness::{Hostility, TestGroup, TestRelay};

fn lost_reason(group: &TestGroup, index: usize) -> Option<LostReason> {
    match group.members[index].lost() {
        Some(ProtectedGroupEvent::Lost(reason)) => Some(*reason),
        _ => None,
    }
}

fn assert_refused(group: &TestGroup, joiner: usize, admitter: usize) {
    assert_eq!(lost_reason(group, joiner), Some(LostReason::WrongPassword));
    assert!(!group.members[joiner].joined());
    assert!(!group.members[joiner].gate.has_keys());
    assert_eq!(group.roster(admitter).len(), 1);
}

#[test]
fn relay_injecting_its_own_welcome_is_refused() {
    let mut group = TestGroup::with_hostility(Hostility::InjectWelcome);
    let alice = group.create("Alice");
    let bob = group.join("Bob");

    assert_refused(&group, bob, alice);
}

#[test]
fn relay_swapping_the_key_package_is_refused() {
    let mut group = TestGroup::with_hostility(Hostility::SwapKeyPackage);
    let alice = group.create("Alice");
    let bob = group.join("Bob");

    assert_refused(&group, bob, alice);
}

#[test]
fn relay_routing_pake_to_itself_gains_nothing() {
    let mut group = TestGroup::with_hostility(Hostility::PakeToSelf);
    let alice = group.create("Alice");
    let bob = group.join("Bob");

    assert_eq!(group.relay.impostor_verified_an_offer, Some(false));
    assert_refused(&group, bob, alice);
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
    let refused = GroupKeyPayload::Admission(AdmissionMessage::Refused)
        .to_bytes()
        .unwrap();
    let bob_socket = group.members[bob].socket;
    group.relay.inject(
        bob_socket,
        GroupKeyFrame::Delivered {
            from: TestRelay::IMPOSTOR,
            body: refused,
        },
    );

    group.pump();
    group.advance(Duration::from_secs(1));

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
