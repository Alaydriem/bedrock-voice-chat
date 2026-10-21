use std::time::Instant;

use bvc_client_lib::groups::protected::{MlsGroupState, SframeCipher};

const OPUS: &[u8] = &[0xAB; 80];

fn identity(canonical: &str) -> common::PlayerIdentity {
    canonical.parse().expect("canonical identity")
}

/// A sender's cipher keyed for its member's current epoch, as the gate installs it.
fn cipher_for(member: &MlsGroupState, now: Instant) -> SframeCipher {
    let mut cipher = SframeCipher::new(member.own_leaf());
    cipher.install_epoch(member.epoch(), member.sframe_base_keys(), now);
    cipher
}

/// Adds Carol to Alice and Bob's group; every member ends at the Add's epoch.
fn add_carol(alice: &mut MlsGroupState, bob: &mut MlsGroupState) -> MlsGroupState {
    let (pending, key_package) = MlsGroupState::key_package(&identity("minecraft:Carol")).unwrap();
    let add = alice.add(&key_package).unwrap();
    alice.merge_pending().unwrap();
    bob.process(&add.commit).unwrap();
    pending.join(&add.welcome).unwrap()
}

/// Alice creates the group and admits Bob; both are returned at the same epoch.
fn alice_and_bob() -> (MlsGroupState, MlsGroupState) {
    let mut alice = MlsGroupState::create(&identity("minecraft:Alice")).unwrap();
    let (pending, key_package) = MlsGroupState::key_package(&identity("minecraft:Bob")).unwrap();
    let staged = alice.add(&key_package).unwrap();
    alice.merge_pending().unwrap();
    let bob = pending.join(&staged.welcome).unwrap();
    (alice, bob)
}

#[test]
fn added_member_derives_the_same_sframe_keys() {
    let (alice, bob) = alice_and_bob();

    assert_eq!(alice.epoch(), bob.epoch());
    assert_eq!(alice.sframe_base_keys(), bob.sframe_base_keys());
    assert_eq!(alice.sframe_base_keys().len(), 2);
}

// Each sender's key is its own, not a shared secret told apart only by the key id.
#[test]
fn every_sender_gets_a_distinct_base_key() {
    let (alice, _bob) = alice_and_bob();
    let keys = alice.sframe_base_keys();

    assert_eq!(keys.len(), 2);
    assert_ne!(keys[0].1, keys[1].1);
}

// Leaving must not leave the epoch's secrets in freed memory.
#[test]
fn wiping_the_group_state_empties_its_store() {
    let (mut alice, _bob) = alice_and_bob();
    assert!(!alice.storage_is_empty());

    alice.wipe();

    assert!(alice.storage_is_empty());
}

#[test]
fn roster_names_both_members_by_canonical_identity() {
    let (alice, bob) = alice_and_bob();

    let names: Vec<String> = bob.roster().iter().map(|m| m.identity.to_string()).collect();

    assert_eq!(names, vec!["minecraft:Alice", "minecraft:Bob"]);
    assert_eq!(alice.own_leaf(), 0);
    assert_eq!(bob.own_leaf(), 1);
}

#[test]
fn epoch_advances_on_add_remove_and_self_update() {
    let (mut alice, mut bob) = alice_and_bob();
    let start = alice.epoch();

    let update = bob.self_update().unwrap();
    bob.merge_pending().unwrap();
    alice.process(&update).unwrap();
    assert_eq!(alice.epoch(), start + 1);
    assert_eq!(bob.epoch(), start + 1);

    let remove = alice.remove(1).unwrap();
    alice.merge_pending().unwrap();
    assert_eq!(alice.epoch(), start + 2);
    assert_eq!(alice.roster().len(), 1);
    let _ = remove;
}

#[test]
fn safety_code_matches_across_members_and_differs_after_a_commit() {
    let (mut alice, mut bob) = alice_and_bob();
    let before = alice.safety_code().digits();
    assert_eq!(before, bob.safety_code().digits());
    assert_eq!(before.split(' ').count(), 6);
    assert!(before.split(' ').all(|group| group.len() == 5));

    let update = alice.self_update().unwrap();
    alice.merge_pending().unwrap();
    bob.process(&update).unwrap();

    assert_eq!(alice.safety_code().digits(), bob.safety_code().digits());
    assert_ne!(alice.safety_code().digits(), before);
}

#[test]
fn removed_member_cannot_process_the_next_commit() {
    let (mut alice, mut bob) = alice_and_bob();
    let (pending, carol_kp) = MlsGroupState::key_package(&identity("minecraft:Carol")).unwrap();
    let add_carol = alice.add(&carol_kp).unwrap();
    alice.merge_pending().unwrap();
    bob.process(&add_carol.commit).unwrap();
    let mut carol = pending.join(&add_carol.welcome).unwrap();

    let remove_bob = alice.remove(1).unwrap();
    alice.merge_pending().unwrap();
    carol.process(&remove_bob).unwrap();
    bob.process(&remove_bob).unwrap();
    assert!(!bob.is_active());

    let update = alice.self_update().unwrap();
    alice.merge_pending().unwrap();

    assert!(bob.process(&update).is_err());
    assert!(carol.process(&update).is_ok());
}

// A Welcome hands over the epoch it creates and nothing older, so audio sealed before an Add
// stays closed to the member it added, even when the server delivers that audio to it.
#[test]
fn a_joiner_cannot_open_audio_sealed_before_it_joined() {
    let now = Instant::now();
    let (mut alice, mut bob) = alice_and_bob();
    let before = cipher_for(&alice, now).seal(OPUS).unwrap();

    let carol = add_carol(&mut alice, &mut bob);
    let mut carol_cipher = cipher_for(&carol, now);
    let after = cipher_for(&alice, now).seal(OPUS).unwrap();

    assert!(carol_cipher.open(&before, now).is_err());
    assert_eq!(carol_cipher.open(&after, now).unwrap().opus, OPUS);
}

// A removed member keeps every key it held and is still handed the commit that removed it, as a
// server that kept forwarding to it would. Neither is enough to open what is sealed afterwards.
#[test]
fn a_removed_member_cannot_open_audio_sealed_after_its_removal() {
    let now = Instant::now();
    let (mut alice, mut bob) = alice_and_bob();
    let mut carol = add_carol(&mut alice, &mut bob);
    let mut carol_cipher = cipher_for(&carol, now);
    let carol_keys = carol.sframe_base_keys();
    assert_eq!(carol_cipher.open(&cipher_for(&alice, now).seal(OPUS).unwrap(), now).unwrap().opus, OPUS);

    let remove = alice.remove(carol.own_leaf()).unwrap();
    alice.merge_pending().unwrap();
    bob.process(&remove).unwrap();
    let _ = carol.process(&remove);
    let after = cipher_for(&alice, now).seal(OPUS).unwrap();
    let new_keys = alice.sframe_base_keys();

    assert!(carol_cipher.open(&after, now).is_err());
    assert!(new_keys.iter().all(|(_, new)| carol_keys.iter().all(|(_, held)| held != new)));
    assert!(carol.sframe_base_keys().iter().all(|(_, derived)| new_keys.iter().all(|(_, new)| new != derived)));
}

#[test]
fn a_tampered_key_package_is_refused() {
    let mut alice = MlsGroupState::create(&identity("minecraft:Alice")).unwrap();
    let (_pending, mut key_package) =
        MlsGroupState::key_package(&identity("minecraft:Bob")).unwrap();
    let middle = key_package.len() / 2;
    key_package[middle] ^= 0xFF;

    assert!(alice.add(&key_package).is_err());
}
