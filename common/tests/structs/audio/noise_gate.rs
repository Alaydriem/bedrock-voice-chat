use common::structs::audio::NoiseGateSettings;

/// The invariant, asserted against the numbers we actually ship rather than against
/// numbers a test made up. A default that cannot shut is a noise gate that does nothing
/// for every user who never opens the settings screen.
#[test]
fn the_default_gate_can_shut() {
    assert!(
        NoiseGateSettings::default().can_close(),
        "the default close threshold must sit below the open threshold"
    );
}

#[test]
fn a_close_threshold_above_the_open_one_is_rejected() {
    let inverted = NoiseGateSettings {
        open_threshold: -50.0,
        close_threshold: -40.0,
        ..Default::default()
    };

    assert!(!inverted.can_close());
}

/// Equal thresholds leave no band for the gate to release in.
#[test]
fn equal_thresholds_are_rejected() {
    let flat = NoiseGateSettings {
        open_threshold: -40.0,
        close_threshold: -40.0,
        ..Default::default()
    };

    assert!(!flat.can_close());
}
