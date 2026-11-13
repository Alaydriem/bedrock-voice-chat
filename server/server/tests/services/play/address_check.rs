use std::net::{IpAddr, Ipv4Addr};

use bvc_server_lib::services::play::{PlayAddressCheck, PlayAddressProblem};
use common::structs::network::AddressClass;

#[test]
fn a_configured_private_address_is_refused() {
    let address: IpAddr = "10.57.2.4".parse().unwrap();

    assert_eq!(
        PlayAddressCheck::check(Some(address)),
        Err(PlayAddressProblem::ConfiguredNotPublic {
            address,
            class: AddressClass::Private,
        })
    );
}

#[test]
fn a_configured_cgnat_address_is_refused() {
    let address: IpAddr = "100.64.0.9".parse().unwrap();

    assert_eq!(
        PlayAddressCheck::check(Some(address)),
        Err(PlayAddressProblem::ConfiguredNotPublic {
            address,
            class: AddressClass::Cgnat,
        })
    );
}

#[test]
fn a_configured_public_address_passes() {
    let configured: IpAddr = "8.8.8.8".parse().unwrap();

    assert_eq!(PlayAddressCheck::check(Some(configured)), Ok(()));
}

#[test]
fn no_configured_address_does_not_stop_play_mode() {
    assert_eq!(PlayAddressCheck::check(None), Ok(()));
}

#[test]
fn a_configured_public_address_silences_the_interface_advisory() {
    let configured: IpAddr = "8.8.8.8".parse().unwrap();

    assert_eq!(
        PlayAddressCheck::advisory(Some(configured), Some(Ipv4Addr::new(100, 64, 0, 2))),
        None
    );
}

#[test]
fn no_address_on_a_cgnat_interface_is_advised_not_refused() {
    let interface = Ipv4Addr::new(100, 64, 0, 2);

    assert_eq!(
        PlayAddressCheck::advisory(None, Some(interface)),
        Some(PlayAddressProblem::BehindCgnat { interface })
    );
    assert_eq!(PlayAddressCheck::check(None), Ok(()));
}

#[test]
fn no_address_on_a_private_interface_has_no_advisory() {
    assert_eq!(
        PlayAddressCheck::advisory(None, Some(Ipv4Addr::new(192, 168, 1, 20))),
        None
    );
}

#[test]
fn no_default_route_has_no_advisory() {
    assert_eq!(PlayAddressCheck::advisory(None, None), None);
}
