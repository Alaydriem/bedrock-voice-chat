use bvc_relay::peer::AdmissionControl;

#[test]
fn a_slot_is_granted_while_capacity_remains() {
    let control = AdmissionControl::new(2);

    let first = control.try_admit().expect("first slot");
    let second = control.try_admit().expect("second slot");

    assert_eq!(control.in_flight(), 2);
    drop(first);
    drop(second);
}

#[test]
fn admission_is_refused_at_the_cap() {
    let control = AdmissionControl::new(1);

    let _held = control.try_admit().expect("first slot");

    assert!(
        control.try_admit().is_none(),
        "an unauthorized caller must not be able to exceed the cap"
    );
}

// Every refusal path drops its slot rather than releasing it explicitly, so the
// release has to be the drop or one forgotten path leaks until restart.
#[test]
fn dropping_a_slot_returns_capacity() {
    let control = AdmissionControl::new(1);

    {
        let _held = control.try_admit().expect("first slot");
        assert!(control.try_admit().is_none());
    }

    assert_eq!(control.in_flight(), 0);
    assert!(control.try_admit().is_some(), "capacity must come back");
}

#[test]
fn a_zero_cap_admits_nobody() {
    let control = AdmissionControl::new(0);

    assert!(control.try_admit().is_none());
}
