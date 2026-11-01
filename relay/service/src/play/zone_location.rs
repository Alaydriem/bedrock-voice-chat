// Where a queried name sits relative to the play zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZoneLocation {
    // The zone's own name.
    Apex,
    // Below the zone. Holds everything left of the zone, which is a single label for
    // every name the registry publishes.
    Host(String),
    // Not in the zone.
    Outside,
}
