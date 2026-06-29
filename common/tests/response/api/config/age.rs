use common::response::ApiConfigAge;

#[test]
fn zero_disables_enforcement() {
    assert_eq!(ApiConfigAge::from_minimum(0).minimum, None);
}

#[test]
fn clamps_below_floor_up_to_thirteen() {
    assert_eq!(ApiConfigAge::from_minimum(9).minimum, Some(13));
}

#[test]
fn clamps_above_ceiling_down_to_eighteen() {
    assert_eq!(ApiConfigAge::from_minimum(21).minimum, Some(18));
}

#[test]
fn keeps_in_range_value() {
    assert_eq!(ApiConfigAge::from_minimum(16).minimum, Some(16));
}
