use common::structs::network::AddressClass;

#[test]
fn each_range_takes_its_class() {
    for (address, class) in [
        ("8.8.8.8", AddressClass::Public),
        ("10.57.2.4", AddressClass::Private),
        ("172.31.255.255", AddressClass::Private),
        ("192.168.1.1", AddressClass::Private),
        ("127.0.0.1", AddressClass::Loopback),
        ("169.254.169.254", AddressClass::LinkLocal),
        ("203.0.113.10", AddressClass::Reserved),
        ("240.0.0.1", AddressClass::Reserved),
        ("2606:4700::1111", AddressClass::Public),
        ("fd00::1", AddressClass::Private),
        ("fe80::1", AddressClass::LinkLocal),
        ("::1", AddressClass::Loopback),
        ("2001:db8::1", AddressClass::Reserved),
    ] {
        assert_eq!(AddressClass::of(address.parse().unwrap()), class, "{address}");
    }
}

#[test]
fn cgnat_boundaries_are_exact() {
    assert_eq!(AddressClass::of("100.63.255.255".parse().unwrap()), AddressClass::Public);
    assert_eq!(AddressClass::of("100.64.0.0".parse().unwrap()), AddressClass::Cgnat);
    assert_eq!(AddressClass::of("100.127.255.255".parse().unwrap()), AddressClass::Cgnat);
    assert_eq!(AddressClass::of("100.128.0.0".parse().unwrap()), AddressClass::Public);
}

#[test]
fn ipv4_mapped_addresses_take_the_ipv4_class() {
    assert_eq!(AddressClass::of("::ffff:10.57.2.4".parse().unwrap()), AddressClass::Private);
    assert_eq!(AddressClass::of("::ffff:100.64.1.1".parse().unwrap()), AddressClass::Cgnat);
    assert_eq!(AddressClass::of("::ffff:8.8.8.8".parse().unwrap()), AddressClass::Public);
}

#[test]
fn ipv6_forms_that_carry_an_ipv4_address_take_its_class() {
    for (address, class) in [
        ("64:ff9b::10.0.0.1", AddressClass::Private),
        ("64:ff9b::8.8.8.8", AddressClass::Public),
        ("64:ff9b:1::8.8.8.8", AddressClass::Reserved),
        ("2002:0a00:0001::1", AddressClass::Private),
        ("2002:0808:0808::1", AddressClass::Public),
        ("::10.0.0.1", AddressClass::Reserved),
        ("::8.8.8.8", AddressClass::Reserved),
        ("100::1", AddressClass::Reserved),
    ] {
        assert_eq!(AddressClass::of(address.parse().unwrap()), class, "{address}");
    }
}
