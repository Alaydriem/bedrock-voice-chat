use common::PlayerSource;

#[test]
fn display_renders_lowercase_variant_names() {
    assert_eq!(PlayerSource::Proximity.to_string(), "proximity");
    assert_eq!(PlayerSource::Group.to_string(), "group");
}
