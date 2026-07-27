use common::structs::network::QuicPortSelection;

#[test]
fn advertised_list_is_tried_in_operator_order() {
    let ports = QuicPortSelection::resolve(&[443, 8443], 8443, None);
    assert_eq!(
        ports,
        vec![443u16, 8443],
        "the operator orders the advertised list by preference; resolution must not reorder it"
    );
}

#[test]
fn scalar_and_cache_are_appended_without_duplicates() {
    let ports = QuicPortSelection::resolve(&[443], 8443, Some("443"));
    assert_eq!(
        ports,
        vec![443u16, 8443],
        "the scalar extends the list, and a cached port already present must not repeat"
    );
}

#[test]
fn cached_value_is_a_last_resort_not_a_preference() {
    let ports = QuicPortSelection::resolve(&[443, 8443], 0, Some("9999"));
    assert_eq!(
        ports,
        vec![443u16, 8443, 9999],
        "a stale cached port must never preempt what the server currently advertises"
    );
}

#[test]
fn zero_ports_are_discarded_as_unreported() {
    let ports = QuicPortSelection::resolve(&[], 0, Some("8443"));
    assert_eq!(
        ports,
        vec![8443u16],
        "a legacy server reports 0 for an unknown port; 0 is not dialable"
    );
}

#[test]
fn exhausted_inputs_fall_back_to_the_default_port() {
    let ports = QuicPortSelection::resolve(&[], 0, None);
    assert_eq!(
        ports,
        vec![QuicPortSelection::DEFAULT_PORT],
        "a client with no information must still have something to dial"
    );
}

#[test]
fn unparseable_cache_is_ignored_rather_than_fatal() {
    let ports = QuicPortSelection::resolve(&[8443], 0, Some("not-a-port"));
    assert_eq!(ports, vec![8443u16]);
}

#[test]
fn out_of_range_ports_are_discarded() {
    let ports = QuicPortSelection::resolve(&[70000, 8443], 0, None);
    assert_eq!(
        ports,
        vec![8443u16],
        "a config value above the u16 ceiling is not a port and must not abort resolution"
    );
}
