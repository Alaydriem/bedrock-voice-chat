use bvc_server_lib::services::HostCapability;

fn report(fetch: &str, write: &str) -> String {
    format!(
        r#"{{"variant":"fat","platform":"linux-x64","mod_version":"1.2.3","fetch":"{fetch}","write":"{write}"}}"#
    )
}

// The reported vocabulary is closed. A value outside it means a mod version this
// server does not understand, and accepting it would put unbounded strings from a
// third-party jar into the metrics pipeline.
#[test]
fn an_outcome_outside_the_known_vocabulary_is_refused() {
    assert!(HostCapability::parse(&report("ok", "ok")).is_ok());
    assert!(HostCapability::parse(&report("nonsense", "ok")).is_err());
    assert!(HostCapability::parse(&report("ok", "nonsense")).is_err());
}

#[test]
fn an_unknown_variant_or_platform_is_refused() {
    let bad_variant =
        r#"{"variant":"enormous","platform":"linux-x64","mod_version":"1.2.3","fetch":"ok","write":"ok"}"#;
    let bad_platform =
        r#"{"variant":"fat","platform":"solaris-sparc","mod_version":"1.2.3","fetch":"ok","write":"ok"}"#;

    assert!(HostCapability::parse(bad_variant).is_err());
    assert!(HostCapability::parse(bad_platform).is_err());
}

// `http_<status>` is open-ended by design, so it is checked structurally rather
// than against a fixed list of statuses.
#[test]
fn an_http_status_outcome_is_accepted_but_a_malformed_one_is_not() {
    let ok =
        r#"{"variant":"skinny","platform":"linux-x64","mod_version":"1.2.3","fetch":"http_403","write":"skipped"}"#;
    let malformed =
        r#"{"variant":"skinny","platform":"linux-x64","mod_version":"1.2.3","fetch":"http_teapot","write":"skipped"}"#;

    assert!(HostCapability::parse(ok).is_ok());
    assert!(HostCapability::parse(malformed).is_err());
}

// The whole point of measuring write separately: a host that fetches but cannot
// write cannot run the skinny jar, and must be distinguishable from one that can.
#[test]
fn a_fetch_ok_write_failure_is_a_valid_and_distinct_report() {
    let parsed = HostCapability::parse(&report("ok", "permission_denied")).expect("valid");

    assert_eq!(parsed.fetch, "ok");
    assert_eq!(parsed.write, "permission_denied");
}
