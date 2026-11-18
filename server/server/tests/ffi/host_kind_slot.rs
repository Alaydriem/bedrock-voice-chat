use bvc_server_lib::ffi::HostKindSlot;
use bvc_server_lib::services::HostKind;

#[test]
fn the_last_value_set_before_start_wins() {
    let slot = HostKindSlot::new();
    slot.set("fabric").unwrap();
    slot.set("powernukkitx").unwrap();
    assert_eq!(slot.seal(), HostKind::PowerNukkitX);
}

#[test]
fn a_set_after_seal_is_refused_and_does_not_change_the_kind() {
    let slot = HostKindSlot::new();
    slot.set("paper").unwrap();
    slot.seal();
    assert!(slot.set("fabric").is_err());
    assert_eq!(slot.seal(), HostKind::Paper);
}

#[test]
fn an_empty_kind_over_ffi_is_unknown_not_standalone() {
    let slot = HostKindSlot::new();
    slot.set("paper").unwrap();
    slot.set("  ").unwrap();
    assert_eq!(slot.seal(), HostKind::Unknown);
}

#[test]
fn an_unset_slot_seals_as_unknown() {
    assert_eq!(HostKindSlot::new().seal(), HostKind::Unknown);
}
