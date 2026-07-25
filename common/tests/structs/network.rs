use common::structs::network::QuicCloseCode;

// The numeric value is a cross-version wire contract: an older client must still
// recognize a rejection from a newer server. Changing it is a breaking protocol
// change, so this pins it deliberately.
#[test]
fn unauthorized_code_value_is_stable() {
    assert_eq!(QuicCloseCode::Unauthorized.as_u64(), 4001);
}

#[test]
fn unauthorized_code_round_trips_from_the_wire_value() {
    assert_eq!(
        QuicCloseCode::from_u64(4001),
        Some(QuicCloseCode::Unauthorized)
    );
}

// An unrecognized code must not be coerced into a known one — the client has to be
// able to tell "rejected" apart from "some other close".
#[test]
fn unknown_code_is_not_recognized() {
    assert_eq!(QuicCloseCode::from_u64(9999), None);
}
