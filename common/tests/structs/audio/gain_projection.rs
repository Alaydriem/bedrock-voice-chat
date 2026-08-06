use common::structs::audio::{GainProjection, PlayerGainSettings, PlayerGainStore};

fn muted() -> PlayerGainSettings {
    PlayerGainSettings {
        gain: 1.0,
        muted: true,
        last_seen: None,
    }
}

// The store is loaded at startup, before any device has been heard from. A projection that only
// rebuilt on a store write left every persisted mute inert until the user touched a slider.
#[test]
fn a_device_seen_after_the_store_was_loaded_still_gets_its_settings() {
    let projection = GainProjection::new();
    let mut store = PlayerGainStore::default();
    store.0.insert("minecraft:Alaydriem".to_string(), muted());
    projection.set_store(store);

    projection.observe(42, "minecraft:Alaydriem");

    assert!(projection.settings_for(42).muted);
}

// A device nobody has an opinion about plays at unity gain, unmuted. This is the correct
// default, and it is also why a wrong key is silent rather than loud.
#[test]
fn an_unknown_device_plays_at_unity_gain() {
    let projection = GainProjection::new();
    let settings = projection.settings_for(7);
    assert_eq!(settings.gain, 1.0);
    assert!(!settings.muted);
}

// One player on two devices carries one opinion, so muting them mutes both.
#[test]
fn two_devices_of_one_player_share_its_settings() {
    let projection = GainProjection::new();
    let mut store = PlayerGainStore::default();
    store.0.insert("minecraft:Alaydriem".to_string(), muted());
    projection.set_store(store);

    projection.observe(1, "minecraft:Alaydriem");
    projection.observe(2, "minecraft:Alaydriem");

    assert!(projection.settings_for(1).muted);
    assert!(projection.settings_for(2).muted);
}

// A bare gamertag is not an identity. Pinned so the two forms cannot drift back apart.
#[test]
fn a_bare_gamertag_does_not_resolve() {
    let projection = GainProjection::new();
    let mut store = PlayerGainStore::default();
    store.0.insert("Alaydriem".to_string(), muted());
    projection.set_store(store);

    projection.observe(1, "minecraft:Alaydriem");

    assert!(!projection.settings_for(1).muted);
}

// A store written after a device was already speaking must reach that device. This is the same
// staleness as the first test from the other direction: whichever input arrives second, the
// answer has to be derived from both rather than frozen when one of them landed.
#[test]
fn a_store_written_after_a_device_was_observed_reaches_it() {
    let projection = GainProjection::new();
    projection.observe(9, "minecraft:Alaydriem");

    let mut store = PlayerGainStore::default();
    store.0.insert("minecraft:Alaydriem".to_string(), muted());
    projection.set_store(store);

    assert!(projection.settings_for(9).muted);
}

// A device that reconnects gets a new connection id, and the old entry must not keep answering
// for it. Re-observing the same device under a different player replaces the mapping rather
// than accumulating a second one.
#[test]
fn re_observing_a_device_replaces_its_player() {
    let projection = GainProjection::new();
    let mut store = PlayerGainStore::default();
    store.0.insert("minecraft:Alaydriem".to_string(), muted());
    projection.set_store(store);

    projection.observe(1, "minecraft:Alaydriem");
    assert!(projection.settings_for(1).muted);

    projection.observe(1, "minecraft:Somebody");
    assert!(
        !projection.settings_for(1).muted,
        "the stale player's mute must not follow a reused device id"
    );
}
