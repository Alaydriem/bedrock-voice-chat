use bvc_client_lib::audio::recording::ParticipantIndex;

#[test]
fn a_name_is_new_only_the_first_time_it_is_seen() {
    let mut index = ParticipantIndex::new();

    assert!(index.observe("minecraft:Petra"));
    assert!(!index.observe("minecraft:Petra"));
}

#[test]
fn jukebox_emitters_are_kept_apart_from_players() {
    let mut index = ParticipantIndex::new();
    index.observe("minecraft:Petra");
    index.observe(&format!(
        "{}rain",
        common::consts::audio::JUKEBOX_PLAYER_PREFIX
    ));

    assert_eq!(index.players(), vec!["minecraft:Petra".to_string()]);
    assert_eq!(index.jukebox().len(), 1);
}

// Your own audio arrives on the input path. Classifying it is the whole fix: without it
// the manifest never names the one track that is always present.
#[test]
fn your_own_emitter_is_a_player_like_any_other() {
    let mut index = ParticipantIndex::new();

    assert!(index.observe("minecraft:Alaydriem"));
    assert_eq!(index.players(), vec!["minecraft:Alaydriem".to_string()]);
}
