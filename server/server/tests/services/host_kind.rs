use bvc_server_lib::services::HostKind;

#[test]
fn known_values_parse_regardless_of_case_and_padding() {
    assert_eq!(HostKind::parse("docker"), HostKind::Docker);
    assert_eq!(HostKind::parse(" Docker "), HostKind::Docker);
    assert_eq!(HostKind::parse("powernukkitx"), HostKind::PowerNukkitX);
}

#[test]
fn empty_is_standalone_and_garbage_is_unknown() {
    assert_eq!(HostKind::parse(""), HostKind::Standalone);
    assert_eq!(HostKind::parse("kubernetes; drop table"), HostKind::Unknown);
}
