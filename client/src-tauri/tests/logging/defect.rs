use bvc_client_lib::logging::Defect;
use curia::Fields;

#[test]
fn an_ordinary_error_declares_no_defect() {
    let mut fields = Fields::new();
    fields.insert("error", "connection reset");

    assert!(Defect::from_fields(&fields).is_none());
}

#[test]
fn a_declared_defect_is_recovered_from_its_field() {
    let mut fields = Fields::new();
    fields.insert("defect", Defect::AudioDeviceLost);

    assert_eq!(Defect::from_fields(&fields), Some(Defect::AudioDeviceLost));
}

#[test]
fn an_unknown_defect_value_is_not_promoted_to_an_issue() {
    let mut fields = Fields::new();
    fields.insert("defect", "SomethingInvented");

    assert!(Defect::from_fields(&fields).is_none());
}

#[test]
fn names_and_variants_do_not_drift() {
    assert_eq!(Defect::NAMES.len(), Defect::all().len());

    for name in Defect::NAMES {
        assert!(
            Defect::parse(name).is_some(),
            "{name} is in NAMES but does not parse"
        );
    }

    for defect in Defect::all() {
        assert!(
            Defect::NAMES.contains(&defect.as_str()),
            "{:?} is a variant missing from NAMES",
            defect
        );
    }
}
