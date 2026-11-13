use common::net::srv::DevPlayNameservers;

#[test]
fn an_unset_value_is_no_override() {
    assert_eq!(DevPlayNameservers::from_value(None), None);
}

#[test]
fn blank_and_whitespace_entries_are_ignored() {
    assert_eq!(DevPlayNameservers::from_value(Some("")), None);
    assert_eq!(DevPlayNameservers::from_value(Some(" , ,, ")), None);
}

#[test]
fn several_entries_keep_their_order_and_ports() {
    assert_eq!(
        DevPlayNameservers::from_value(Some("127.0.0.1:5300, ns.test:5301 ,,10.0.0.1:53")),
        Some(vec![
            "127.0.0.1:5300".to_string(),
            "ns.test:5301".to_string(),
            "10.0.0.1:53".to_string(),
        ])
    );
}

#[test]
fn an_entry_without_a_port_gets_53() {
    assert_eq!(
        DevPlayNameservers::from_value(Some("127.0.0.1,ns.test,::1,[::1]:5300")),
        Some(vec![
            "127.0.0.1:53".to_string(),
            "ns.test:53".to_string(),
            "[::1]:53".to_string(),
            "[::1]:5300".to_string(),
        ])
    );
}
