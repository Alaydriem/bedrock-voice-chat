use std::time::Duration;

use bvc_client_lib::groups::protected::{
    AdmissionRateLimiter, CodeRole, LostReason, ProtectedGroupEvent,
};
use common::structs::group_key::{AdmissionMessage, GroupKeyFrame, GroupKeyPayload};

use super::harness::TestGroup;

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
fn allow_and_it_matches_admits_the_joiner() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();

    assert_eq!(group.members[alice].requests(), vec![(group.members[bob].socket, "minecraft:Bob".to_string())]);
    group.admit(alice, bob);

    assert!(group.members[bob].joined());
    assert_eq!(group.safety_code(alice), group.safety_code(bob));
    let bob_socket = group.members[bob].socket;
    assert!(group.members[alice].has(|e| *e == ProtectedGroupEvent::Admitted { joiner: bob_socket }));
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
fn member_denies_and_joiner_loses_with_refused() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();
    group.check(alice, bob);
    let (check_id, _) = group.member_code(alice, bob).unwrap();

    group.decide(alice, bob, check_id, false);

    assert_eq!(lost_reason(&group, bob), Some(LostReason::Refused));
    assert_eq!(group.roster(alice), vec!["minecraft:Alice"]);
}

#[test]
fn joiner_says_it_does_not_match_and_loses_with_mismatch() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();
    group.check(alice, bob);
    let (check_id, _) = group.joiner_code(bob).unwrap();

    group.confirm(bob, check_id, false);

    assert_eq!(lost_reason(&group, bob), Some(LostReason::Mismatch));
    assert!(!group.members[bob].gate.has_keys());
}

// One member checks; every other member's card says so, and they cannot check the same joiner.
#[test]
fn two_members_check_and_the_second_sees_checking() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join_quietly("Carol");
    group.pump();
    let carol_socket = group.members[carol].socket;

    group.check(alice, carol);

    assert!(group.members[bob].has(|e| *e
        == ProtectedGroupEvent::Checking {
            joiner: carol_socket,
            member_name: "minecraft:Alice".to_string(),
        }));
    assert!(group.members[bob].service.check(carol_socket).is_err());
}

// Two members press Check before either hears of the other. The joiner answers the first and
// tells the second it is busy; only one code exists.
#[test]
fn two_members_checking_at_once_gives_one_code_and_one_busy() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join_quietly("Carol");
    group.pump();
    let carol_socket = group.members[carol].socket;

    group.members[alice].service.check(carol_socket).unwrap();
    group.members[bob].service.check(carol_socket).unwrap();
    group.pump();

    let codes = [alice, bob]
        .iter()
        .filter(|m| group.member_code(**m, carol).is_some())
        .count();
    let ended = [alice, bob]
        .iter()
        .filter(|m| {
            group.members[**m].has(|e| *e == ProtectedGroupEvent::RequestEnded { joiner: carol_socket })
        })
        .count();
    assert_eq!(codes, 1);
    assert_eq!(ended, 1);
}

// The other member's "checking" announcement arrives after this member already holds the code.
// The code, Deny and Admit must stay on this member's card.
#[test]
fn the_member_holding_the_code_ignores_a_late_checking() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join_quietly("Carol");
    group.pump();
    let carol_socket = group.members[carol].socket;

    group.members[alice].service.check(carol_socket).unwrap();
    group.members[bob].service.check(carol_socket).unwrap();
    group.pump();

    let winner = if group.member_code(alice, carol).is_some() { alice } else { bob };
    assert!(!group.members[winner].has(|e| matches!(
        e,
        ProtectedGroupEvent::Checking { joiner, .. } if *joiner == carol_socket
    )));
}

#[test]
fn checking_member_disappears_and_the_request_ends_with_expired() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join_quietly("Carol");
    group.pump();
    group.check(alice, carol);

    group.disconnect(alice);

    assert_eq!(lost_reason(&group, carol), Some(LostReason::Expired));
    assert!(!group.roster(bob).contains(&"minecraft:Carol".to_string()));
}

#[test]
fn an_unanswered_request_ends_with_expired_after_two_minutes() {
    let mut group = TestGroup::new();
    let _alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();

    group.advance(Duration::from_secs(119));
    assert_eq!(lost_reason(&group, bob), None);
    group.advance(Duration::from_secs(1));

    assert_eq!(lost_reason(&group, bob), Some(LostReason::Expired));
}

#[test]
fn member_checks_two_joiners_at_once_and_admits_both() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let carol = group.join_quietly("Carol");
    let dave = group.join_quietly("Dave");
    group.pump();
    let start = group.epoch(alice).unwrap();

    group.check(alice, carol);
    group.check(alice, dave);
    let (carol_check, carol_code) = group.member_code(alice, carol).unwrap();
    let (dave_check, dave_code) = group.member_code(alice, dave).unwrap();
    assert_ne!(carol_check, dave_check);
    assert_eq!(group.joiner_code(carol).unwrap().1, carol_code);
    assert_eq!(group.joiner_code(dave).unwrap().1, dave_code);

    let carol_socket = group.members[carol].socket;
    let dave_socket = group.members[dave].socket;
    group.members[alice].service.decide(carol_socket, carol_check, true).unwrap();
    group.members[alice].service.decide(dave_socket, dave_check, true).unwrap();
    group.pump();
    let carol_joiner_check = group.joiner_code(carol).unwrap().0;
    let dave_joiner_check = group.joiner_code(dave).unwrap().0;
    group.confirm(carol, carol_joiner_check, true);
    group.confirm(dave, dave_joiner_check, true);

    assert!(group.members[carol].joined());
    assert!(group.members[dave].joined());
    assert_eq!(group.epoch(alice), Some(start + 2), "each Add has its own epoch");
    assert_converged(&group, &[alice, carol, dave]);
}

#[test]
fn admit_for_one_check_id_never_admits_the_other_joiner() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let carol = group.join_quietly("Carol");
    let dave = group.join_quietly("Dave");
    group.pump();
    group.check(alice, carol);
    group.check(alice, dave);
    let (carol_check, _) = group.member_code(alice, carol).unwrap();
    let dave_socket = group.members[dave].socket;

    let crossed = group.members[alice].service.decide(dave_socket, carol_check, true);
    group.pump();

    assert!(crossed.is_err());
    assert_eq!(group.roster(alice), vec!["minecraft:Alice"]);
}

#[test]
fn second_decide_for_the_same_check_id_is_stale() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();
    group.check(alice, bob);
    let (check_id, _) = group.member_code(alice, bob).unwrap();
    let bob_socket = group.members[bob].socket;

    group.members[alice].service.decide(bob_socket, check_id, true).unwrap();

    assert!(group.members[alice].service.decide(bob_socket, check_id, true).is_err());
}

// The member admits two people back to back; the second Add waits in the queue past the time its
// check would have expired. It still goes out under its own check's id, and that joiner enters.
#[test]
fn queued_add_after_its_check_expires_still_sends_welcome_with_its_own_check_id() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let carol = group.join_quietly("Carol");
    let dave = group.join_quietly("Dave");
    group.pump();
    group.check(alice, carol);
    group.check(alice, dave);
    let (carol_check, _) = group.member_code(alice, carol).unwrap();
    let (dave_check, _) = group.member_code(alice, dave).unwrap();
    let carol_socket = group.members[carol].socket;
    let dave_socket = group.members[dave].socket;

    group.members[alice].service.decide(carol_socket, carol_check, true).unwrap();
    group.members[alice].service.decide(dave_socket, dave_check, true).unwrap();
    group.clock.advance(Duration::from_secs(61));
    group.pump();
    let dave_joiner_check = group.joiner_code(dave).unwrap().0;
    group.confirm(dave, dave_joiner_check, true);

    assert!(group.members[dave].joined());
}

#[test]
fn add_older_than_90_s_is_dropped_and_the_joiner_is_refused() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let carol = group.join_quietly("Carol");
    let dave = group.join_quietly("Dave");
    group.pump();
    group.check(alice, carol);
    group.check(alice, dave);
    let (carol_check, _) = group.member_code(alice, carol).unwrap();
    let (dave_check, _) = group.member_code(alice, dave).unwrap();
    let carol_socket = group.members[carol].socket;
    let dave_socket = group.members[dave].socket;

    group.members[alice].service.decide(carol_socket, carol_check, true).unwrap();
    group.members[alice].service.decide(dave_socket, dave_check, true).unwrap();
    group.clock.advance(Duration::from_secs(91));
    group.pump();

    assert_eq!(lost_reason(&group, dave), Some(LostReason::Refused));
    assert!(!group.roster(alice).contains(&"minecraft:Dave".to_string()));
}

#[test]
fn joiner_leaving_drops_its_queued_add() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let carol = group.join_quietly("Carol");
    let dave = group.join_quietly("Dave");
    group.pump();
    group.check(alice, carol);
    group.check(alice, dave);
    let (carol_check, _) = group.member_code(alice, carol).unwrap();
    let (dave_check, _) = group.member_code(alice, dave).unwrap();
    let carol_socket = group.members[carol].socket;
    let dave_socket = group.members[dave].socket;

    group.members[alice].service.decide(carol_socket, carol_check, true).unwrap();
    group.members[alice].service.decide(dave_socket, dave_check, true).unwrap();
    group.disconnect(dave);
    group.advance(Duration::from_secs(5));

    assert!(!group.roster(alice).contains(&"minecraft:Dave".to_string()));
}

// The joiner's socket closes while its Add is already on the wire. The member that added it
// removes the leaf again, rather than keep a member nobody holds the keys for.
#[test]
fn joiner_leaving_with_its_add_in_flight_is_removed_again() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let dave = group.join_quietly("Dave");
    group.pump();
    group.check(alice, dave);
    let (dave_check, _) = group.member_code(alice, dave).unwrap();
    let dave_socket = group.members[dave].socket;

    group.members[alice].service.decide(dave_socket, dave_check, true).unwrap();
    group.disconnect(dave);
    group.advance(Duration::from_secs(5));

    assert_eq!(group.roster(alice), vec!["minecraft:Alice"]);
}

#[test]
fn fourth_open_check_is_refused() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let joiners: Vec<usize> = ["B", "C", "D", "E"]
        .iter()
        .map(|name| group.join_quietly(name))
        .collect();
    group.pump();

    for joiner in &joiners[..3] {
        group.check(alice, *joiner);
    }
    let fourth = group.members[joiners[3]].socket;

    assert!(group.members[alice].service.check(fourth).is_err());
}

#[test]
fn sixth_request_waits_until_a_card_closes() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let joiners: Vec<usize> = ["B", "C", "D", "E", "F", "G"]
        .iter()
        .map(|name| group.join_quietly(name))
        .collect();
    group.pump();

    assert_eq!(group.members[alice].requests().len(), 5);
    assert!(group.members[alice].has(|e| *e == ProtectedGroupEvent::Waiting(1)));

    group.check(alice, joiners[0]);
    let (check_id, _) = group.member_code(alice, joiners[0]).unwrap();
    group.decide(alice, joiners[0], check_id, false);

    let sixth = group.members[joiners[5]].socket;
    assert!(group.members[alice].requests().iter().any(|(socket, _)| *socket == sixth));
    assert!(group.members[alice].has(|e| *e == ProtectedGroupEvent::Waiting(0)));
}

#[test]
fn a_burst_of_requests_plays_the_sound_once() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    group.join_quietly("Bob");
    group.join_quietly("Carol");
    group.pump();

    let cues: Vec<bool> = group.members[alice]
        .events
        .iter()
        .filter_map(|e| match e {
            ProtectedGroupEvent::Request { cue, .. } => Some(*cue),
            _ => None,
        })
        .collect();
    assert_eq!(cues, vec![true, false]);
}

// A `Busy` names a check and carries no proof of who sent it, so it must never count against the
// joiner: a server could otherwise lock anyone out by forging them. Every round below must still
// be allowed to check; `check` fails the test if the limit was reached.
#[test]
fn forged_busy_does_not_count_toward_the_limit() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");

    for _ in 0..AdmissionRateLimiter::PER_JOINER + 1 {
        let carol = group.join_quietly("Carol");
        group.pump();
        group.check(alice, carol);
        let (check_id, _) = group.member_code(alice, carol).unwrap();
        let carol_socket = group.members[carol].socket;
        let busy = GroupKeyPayload::Admission(AdmissionMessage::Busy { check_id })
            .to_bytes()
            .unwrap();
        let alice_socket = group.members[alice].socket;
        group.relay.inject(
            alice_socket,
            GroupKeyFrame::Delivered {
                from: carol_socket,
                body: busy,
            },
        );
        group.pump();
        group.disconnect(carol);
    }
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
    let _carol = group.join("Carol");
    group.freeze(alice);

    group.disconnect(2);
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

// The member's leaf key is in every open `Check`; the joiner accepts a Welcome only from that key.
// No ratchet runs while a check is open.
#[test]
fn no_periodic_ratchet_runs_while_a_check_is_open() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let start = group.epoch(alice).unwrap();
    group.advance(Duration::from_secs(4 * 60 + 50));
    let bob = group.join_quietly("Bob");
    group.pump();
    group.check(alice, bob);
    group.freeze(bob);

    group.advance(Duration::from_secs(30));
    assert_eq!(group.epoch(alice), Some(start), "ratcheted with a check open");

    group.advance(Duration::from_secs(60));
    assert_eq!(group.epoch(alice), Some(start + 1), "never ratcheted once the check closed");
}

#[test]
fn competing_adds_from_two_members_resolve_by_stale_retry() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join("Bob");
    let carol = group.join_quietly("Carol");
    let dave = group.join_quietly("Dave");
    group.pump();
    group.check(alice, carol);
    group.check(bob, dave);
    let (carol_check, _) = group.member_code(alice, carol).unwrap();
    let (dave_check, _) = group.member_code(bob, dave).unwrap();
    let carol_socket = group.members[carol].socket;
    let dave_socket = group.members[dave].socket;

    group.members[alice].service.decide(carol_socket, carol_check, true).unwrap();
    group.members[bob].service.decide(dave_socket, dave_check, true).unwrap();
    group.pump();
    let carol_joiner_check = group.joiner_code(carol).unwrap().0;
    let dave_joiner_check = group.joiner_code(dave).unwrap().0;
    group.confirm(carol, carol_joiner_check, true);
    group.confirm(dave, dave_joiner_check, true);
    group.advance(Duration::from_secs(10));

    assert!(group.members[carol].joined());
    assert!(group.members[dave].joined());
    assert_converged(&group, &[alice, bob, carol, dave]);
    assert_eq!(group.roster(alice).len(), 4);
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

#[test]
fn a_joiner_holds_no_keys_and_seals_nothing_before_admission() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    group.freeze(alice);
    let bob = group.join("Bob");

    assert!(group.members[bob].gate.is_protected());
    assert!(group.members[bob].gate.seal(OPUS).is_none());
}

#[test]
fn the_joiner_is_shown_the_checking_members_name() {
    let mut group = TestGroup::new();
    let alice = group.create("Alice");
    let bob = group.join_quietly("Bob");
    group.pump();

    group.check(alice, bob);

    assert!(group.members[bob].has(|e| matches!(
        e,
        ProtectedGroupEvent::Code { role: CodeRole::Joiner, peer_name, .. } if peer_name == "minecraft:Alice"
    )));
}
