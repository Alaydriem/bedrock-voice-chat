use super::Destination;

pub struct FieldSpec {
    pub name: &'static str,
    pub destination: Destination,
    // Non-empty only for Destination::Tag. A value outside this set is demoted
    // to an attribute rather than becoming an unbounded tag.
    pub variants: &'static [&'static str],
}
