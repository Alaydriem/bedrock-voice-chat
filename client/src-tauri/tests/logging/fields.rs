use bvc_client_lib::logging::{Destination, Vocabulary};
use curia::Fields;

#[test]
fn declared_tag_fields_become_tags() {
    let mut fields = Fields::new();
    fields.insert("transport", "quic");

    let routed = Vocabulary::route(&fields);

    assert_eq!(
        routed.tags,
        vec![("transport".to_string(), "quic".to_string())]
    );
}

#[test]
fn a_value_outside_the_variant_set_is_demoted_to_an_attribute() {
    let mut fields = Fields::new();
    fields.insert("transport", "carrier-pigeon");

    let routed = Vocabulary::route(&fields);

    assert!(routed.tags.is_empty());
    assert_eq!(
        routed.attributes.get("transport").unwrap(),
        &serde_json::json!("carrier-pigeon")
    );
}

#[test]
fn unbounded_fields_are_attributes_never_tags() {
    let mut fields = Fields::new();
    fields.insert("player_hash", "9f2c1e");
    fields.insert("connected_server", "https://example.invalid");
    fields.insert("device_name", "Focusrite Scarlett 2i2");

    let routed = Vocabulary::route(&fields);

    assert!(routed.tags.is_empty());
    assert_eq!(routed.attributes.len(), 3);
}

#[test]
fn undeclared_fields_land_in_context_rather_than_being_dropped() {
    let mut fields = Fields::new();
    fields.insert("something_new", 7);

    let routed = Vocabulary::route(&fields);

    assert!(routed.tags.is_empty());
    assert!(routed.attributes.is_empty());
    assert_eq!(
        routed.context.get("something_new").unwrap(),
        &serde_json::json!(7)
    );
}

#[test]
fn envelope_keys_are_reserved() {
    for key in ["ts", "level", "target", "msg"] {
        assert!(Vocabulary::is_reserved(key), "{key} must be reserved");
    }
    assert!(!Vocabulary::is_reserved("transport"));
}

#[test]
fn every_declared_tag_field_has_a_variant_set() {
    for field in Vocabulary::declared() {
        if field.destination == Destination::Tag {
            assert!(
                !field.variants.is_empty(),
                "{} is declared Tag with no variant set",
                field.name
            );
        }
    }
}
