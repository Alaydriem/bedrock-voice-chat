use common::structs::audio::NoiseGateStatus;

/// The distinction the readout could not make. A gate that is switched off and a gate that
/// is open both pass audio, so "is signal getting through" answers the same for each — and
/// a row that reported only that could never say whether the gate was attached at all.
#[test]
fn a_gate_that_is_off_is_not_the_same_as_a_gate_that_is_open() {
    assert_eq!(NoiseGateStatus::of(false, true), NoiseGateStatus::Disabled);
    assert_eq!(NoiseGateStatus::of(true, true), NoiseGateStatus::Open);
}

/// Silence with the gate switched off is silence, not a gate holding the mic shut.
/// Reporting "closed" there sends the reader to a setting that is not doing anything.
#[test]
fn silence_with_the_gate_off_is_still_reported_as_off() {
    assert_eq!(NoiseGateStatus::of(false, false), NoiseGateStatus::Disabled);
}

#[test]
fn an_enabled_gate_passing_nothing_is_closed() {
    assert_eq!(NoiseGateStatus::of(true, false), NoiseGateStatus::Closed);
}

/// Only an enabled gate can cut audio, so only an enabled gate is worth suspecting when a
/// microphone goes quiet.
#[test]
fn only_an_enabled_gate_can_be_holding_the_microphone_shut() {
    assert!(NoiseGateStatus::Closed.is_cutting());
    assert!(!NoiseGateStatus::Open.is_cutting());
    assert!(!NoiseGateStatus::Disabled.is_cutting());
}

/// Whether the gate is bound to the audio path at all, which is the question the readout
/// exists to answer at a glance.
#[test]
fn the_status_says_whether_the_gate_is_attached() {
    assert!(NoiseGateStatus::Open.is_enabled());
    assert!(NoiseGateStatus::Closed.is_enabled());
    assert!(!NoiseGateStatus::Disabled.is_enabled());
}
