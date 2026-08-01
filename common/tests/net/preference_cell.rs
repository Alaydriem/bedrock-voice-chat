use common::net::FamilyPreferenceCell;
use common::structs::reachability::AddressFamilyPreference;
use std::sync::Arc;

// Every released client is IPv4-only today. A cell that started out preferring
// IPv6 would change behaviour for hosts no probe had looked at yet.
#[test]
fn a_fresh_cell_prefers_ipv4() {
    let cell = FamilyPreferenceCell::new();

    assert_eq!(cell.get(), AddressFamilyPreference::PreferIpv4);
}

#[test]
fn a_published_verdict_is_visible_through_every_handle() {
    let cell = FamilyPreferenceCell::new_shared();
    let reader = Arc::clone(&cell);

    cell.set(AddressFamilyPreference::PreferIpv6);

    assert_eq!(reader.get(), AddressFamilyPreference::PreferIpv6);

    cell.set(AddressFamilyPreference::PreferIpv4);

    assert_eq!(reader.get(), AddressFamilyPreference::PreferIpv4);
}
