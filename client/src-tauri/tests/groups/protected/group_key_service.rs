use std::time::Duration;

use bvc_client_lib::groups::protected::{
    GroupPassword, JoinerSession, LostReason, ProtectedGroupEvent,
};
use common::structs::group_key::{AdmissionMessage, GroupKeyFrame, GroupKeyPayload};

use super::harness::{Hostility, TestGroup};

const OPUS: &[u8] = &[0x5A; 80];

fn lost_reason(group: &TestGroup, index: usize) -> Option<LostReason> {
    match group.members[index].lost() {
        Some(ProtectedGroupEvent::Lost(reason)) => Some(*reason),
        _ => None,
    }
}

fn assert_converged(group: &TestGroup, members: &[usize]) {
    let first = members[0];
    for &m in &members[1..] {
        assert_eq!(group.epoch(m), group.epoch(first), "epoch of member {m}");
        assert_eq!(group.roster(m), group.roster(first), "roster of member {m}");
        assert_eq!(group.safety_code(m), group.safety_code(first), "safety code of member {m}");
    }
}

#[test]
fn three_members_converge_on_one_epoch_and_roster() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join("Carol");

    assert!(group.members[bob].joined());
    assert!(group.members[carol].joined());
    assert_converged(&group, &[alice, bob, carol]);
    assert_eq!(
        group.roster(alice),
        vec!["minecraft:Alice", "minecraft:Bob", "minecraft:Carol"]
    );
}

#[test]
fn a_member_hears_another_through_the_gate_and_the_speaker_is_named() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");

    let sealed = group.members[alice].gate.seal(OPUS).expect("alice holds keys");

    assert_eq!(
        group.members[bob].gate.open(&sealed, "minecraft:Alice").as_deref(),
        Some(OPUS)
    );
    assert!(group.members[bob].gate.open(&sealed, "minecraft:Mallory").is_none());
}

#[test]
fn competing_adds_resolve_by_stale_retry() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join_quietly("Carol");
    let dave = group.join_quietly("Dave");

    group.pump();
    group.advance(Duration::from_secs(10));

    assert!(group.members[carol].joined());
    assert!(group.members[dave].joined());
    assert_converged(&group, &[alice, bob, carol, dave]);
    assert_eq!(group.roster(alice).len(), 4);
}

#[test]
fn gone_member_is_removed_by_lowest_leaf_at_once() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join("Carol");

    group.disconnect(bob);

    assert_eq!(group.roster(alice), vec!["minecraft:Alice", "minecraft:Carol"]);
    assert_converged(&group, &[alice, carol]);
}

// The commonest shape of all: two people, and the one who made the group leaves. The one who
// remains must learn which socket was the creator's, or the departure is never committed.
#[test]
fn the_creator_leaving_a_two_member_group_is_removed_by_the_joiner() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let before = group.epoch(bob).unwrap();

    group.disconnect(alice);

    assert_eq!(group.roster(bob), vec!["minecraft:Bob"]);
    assert_eq!(group.epoch(bob), Some(before + 1));
}

#[test]
fn gone_member_is_removed_by_the_next_leaf_after_3s_when_the_lowest_is_silent() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join("Carol");
    group.freeze(alice);

    group.disconnect(carol);
    group.advance(Duration::from_secs(2));
    assert!(group.roster(bob).contains(&"minecraft:Carol".to_string()));

    group.advance(Duration::from_secs(2));
    assert!(!group.roster(bob).contains(&"minecraft:Carol".to_string()));
}

#[test]
fn periodic_commit_rotates_through_members_every_5_minutes() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let start = group.epoch(alice).unwrap();

    group.advance(Duration::from_secs(4 * 60));
    assert_eq!(group.epoch(alice), Some(start));

    group.advance(Duration::from_secs(61));
    assert_eq!(group.epoch(alice), Some(start + 1));

    group.advance(Duration::from_secs(5 * 60 + 1));
    assert_eq!(group.epoch(alice), Some(start + 2));
    assert_converged(&group, &[alice, bob]);
}

#[test]
fn the_other_member_commits_the_periodic_ratchet_when_the_designated_one_is_silent() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let start = group.epoch(bob).unwrap();
    // Leaf order rotates by epoch, so whoever is designated now, freezing both in turn and
    // checking that the ratchet still happens covers the fallback either way.
    let designated = if start % 2 == 0 { alice } else { bob };
    let other = if designated == alice { bob } else { alice };
    group.freeze(designated);

    group.advance(Duration::from_secs(5 * 60 + 4));

    assert_eq!(group.epoch(other), Some(start + 1));
}

#[test]
fn joiner_retries_any_three_times_then_reports_admitter_unreachable() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    group.freeze(alice);
    let bob = group.join("Bob");

    group.advance(Duration::from_secs(11));
    assert_eq!(lost_reason(&group, bob), None);

    group.advance(Duration::from_secs(5));
    assert_eq!(lost_reason(&group, bob), Some(LostReason::AdmitterUnreachable));
}

#[test]
fn a_non_admitter_answers_a_pake_init_with_a_redirect_to_the_admitter() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let joiner = group.relay.connect();
    group.relay.take_inbox(joiner);

    let (_session, init) = JoinerSession::start(
        TestGroup::ID,
        &GroupPassword::new(TestGroup::PASSWORD).unwrap(),
    );
    group.relay.inject(
        group.members[bob].socket,
        GroupKeyFrame::Delivered {
            from: joiner,
            body: GroupKeyPayload::Admission(init).to_bytes().unwrap(),
        },
    );
    group.pump();
    let answers: Vec<AdmissionMessage> = group
        .relay
        .take_inbox(joiner)
        .into_iter()
        .filter_map(|frame| match frame {
            GroupKeyFrame::Delivered { body, .. } => match GroupKeyPayload::from_bytes(&body) {
                Ok(GroupKeyPayload::Admission(message)) => Some(message),
                _ => None,
            },
            _ => None,
        })
        .collect();

    assert_eq!(
        answers,
        vec![AdmissionMessage::Redirect {
            to: Some(group.members[alice].socket)
        }]
    );
}

// The relay sends the joiner to a member that is not the admitter. The joiner follows the redirect
// and is admitted by the lowest leaf; the member it first reached never runs the PAKE.
#[test]
fn a_joiner_routed_to_a_non_admitter_is_redirected_and_admitted() {
    let mut group = TestGroup::with_hostility(Hostility::AnyToNewestMember);
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join("Carol");

    assert!(group.members[carol].joined());
    assert_converged(&group, &[alice, bob, carol]);
    let admitter = group.members[alice].socket;
    assert!(group.relay.pake_replies_from.iter().all(|socket| *socket == admitter));
}

#[test]
fn wrong_password_joiner_is_refused_and_never_added() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let mallory = group.join_with("Mallory", "not the password");

    assert_eq!(lost_reason(&group, mallory), Some(LostReason::WrongPassword));
    assert!(!group.members[mallory].joined());
    assert_eq!(group.roster(alice), vec!["minecraft:Alice"]);
}

#[test]
fn no_admitter_close_reason_reports_no_admitter() {
    let mut group = TestGroup::new();
    let bob = group.join("Bob");

    group.members[bob].service.on_socket_closed(Some("NoAdmitterOnline"));
    group.members[bob].collect_events();

    assert_eq!(lost_reason(&group, bob), Some(LostReason::NoAdmitter));
}

#[test]
fn leave_clears_the_gate_and_zeroes_state() {
    let mut group = TestGroup::new();
    let _alice = group.create("Alice");
    let bob = group.join("Bob");
    assert!(group.members[bob].gate.has_keys());

    group.members[bob].service.leave();

    assert!(!group.members[bob].gate.has_keys());
    assert_eq!(group.epoch(bob), None);
    assert!(group.members[bob].gate.seal(OPUS).is_none());
}

#[test]
fn socket_closed_clears_the_gate() {
    let mut group = TestGroup::new();
    let _alice = group.create("Alice");
    let bob = group.join("Bob");

    group.disconnect(bob);

    assert!(!group.members[bob].gate.has_keys());
    assert!(group.members[bob].gate.seal(OPUS).is_none());
    assert_eq!(lost_reason(&group, bob), Some(LostReason::SocketClosed));
}

// A replaced session's runner can still report its own loss after the new session has keys. Its
// cleanup must touch only its own session, or the new group goes silent — or, worse, unprotected.
#[test]
fn a_stale_session_losing_its_group_leaves_a_newer_sessions_keys_alone() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let gate = group.members[alice].gate.clone();
    let (outbound, _outbound_rx) = tokio::sync::mpsc::channel(16);
    let (events, _events_rx) = tokio::sync::mpsc::channel(16);
    let _newer = bvc_client_lib::groups::protected::GroupKeyService::create(
        "EFGH6789".to_string(),
        "minecraft:Alice".parse().unwrap(),
        bvc_client_lib::groups::protected::GroupPassword::new("another one").unwrap(),
        group.clock.clone(),
        outbound,
        events,
        gate.clone(),
    )
    .unwrap();

    group.members[alice].service.on_socket_closed(None);

    assert!(gate.is_protected());
    assert!(gate.has_keys(), "the stale session wiped the newer session's keys");
}

// A second PAKE from a socket that already has one in progress would be a password guess the
// rate limiter never sees, so it is refused rather than allowed to replace the first.
#[test]
fn a_second_pake_init_from_the_same_socket_is_refused() {
    use bvc_client_lib::groups::protected::{GroupKeyService, GroupPassword, JoinerSession, SealGate};
    use common::structs::group_key::{
        AdmissionMessage, GroupKeyFrame, GroupKeyPayload, GroupKeyRecipient,
    };

    let (outbound, mut outbound_rx) = tokio::sync::mpsc::channel(64);
    let (events, _events_rx) = tokio::sync::mpsc::channel(64);
    let password = GroupPassword::new(TestGroup::PASSWORD).unwrap();
    let mut alice = GroupKeyService::create(
        TestGroup::ID.to_string(),
        "minecraft:Alice".parse().unwrap(),
        GroupPassword::new(TestGroup::PASSWORD).unwrap(),
        super::harness::TestClock::new(),
        outbound,
        events,
        std::sync::Arc::new(SealGate::new()),
    )
    .unwrap();
    alice.on_frame(GroupKeyFrame::Welcome { socket: 1 });
    let init = |_: ()| {
        let (_session, init) = JoinerSession::start(TestGroup::ID, &password);
        GroupKeyFrame::Delivered {
            from: 7,
            body: GroupKeyPayload::Admission(init).to_bytes().unwrap(),
        }
    };
    let replies_to_7 = |rx: &mut tokio::sync::mpsc::Receiver<GroupKeyFrame>| {
        let mut replies = Vec::new();
        while let Ok(frame) = rx.try_recv() {
            if let GroupKeyFrame::Direct { to: GroupKeyRecipient::Socket(7), body } = frame {
                if let Ok(GroupKeyPayload::Admission(message)) = GroupKeyPayload::from_bytes(&body) {
                    replies.push(message);
                }
            }
        }
        replies
    };

    alice.on_frame(init(()));
    assert!(matches!(
        replies_to_7(&mut outbound_rx).as_slice(),
        [AdmissionMessage::PakeReply(_)]
    ));

    alice.on_frame(init(()));
    assert_eq!(replies_to_7(&mut outbound_rx), vec![AdmissionMessage::Refused]);
}

#[test]
fn a_joiner_holds_no_keys_and_seals_nothing_before_admission() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    group.freeze(alice);
    let bob = group.join("Bob");

    assert!(group.members[bob].gate.is_protected());
    assert!(group.members[bob].gate.seal(OPUS).is_none());
}
