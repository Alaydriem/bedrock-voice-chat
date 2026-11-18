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

    assert_eq!(parsed.fetch.as_deref(), Some("ok"));
    assert_eq!(parsed.write.as_deref(), Some("permission_denied"));
}

#[test]
fn a_java_report_with_loader_and_mode_is_accepted_and_unknown_values_are_refused() {
    let ok = r#"{"variant":"fat","platform":"linux-x64","mod_version":"1.2.3","fetch":"ok","write":"ok","loader":"powernukkitx","mode":"embedded"}"#;
    let bad_loader = r#"{"variant":"fat","platform":"linux-x64","mod_version":"1.2.3","fetch":"ok","write":"ok","loader":"forge","mode":"embedded"}"#;
    let bad_mode = r#"{"variant":"fat","platform":"linux-x64","mod_version":"1.2.3","fetch":"ok","write":"ok","loader":"paper","mode":"hybrid"}"#;
    assert!(HostCapability::parse(ok).is_ok());
    assert!(HostCapability::parse(bad_loader).is_err());
    assert!(HostCapability::parse(bad_mode).is_err());
}

#[test]
fn a_bds_report_is_accepted_without_jar_fields_and_refused_with_them() {
    let ok = r#"{"mod_version":"1.0.0-beta.28","loader":"bds","mode":"external"}"#;
    let with_jar = r#"{"variant":"fat","mod_version":"1.0.0-beta.28","loader":"bds","mode":"external"}"#;
    let embedded = r#"{"mod_version":"1.0.0-beta.28","loader":"bds","mode":"embedded"}"#;
    assert!(HostCapability::parse(ok).is_ok());
    assert!(HostCapability::parse(with_jar).is_err());
    assert!(HostCapability::parse(embedded).is_err());
}

#[test]
fn loader_without_mode_or_mode_without_loader_is_refused() {
    let no_mode = r#"{"variant":"fat","platform":"linux-x64","mod_version":"1.2.3","fetch":"ok","write":"ok","loader":"paper"}"#;
    let no_loader = r#"{"variant":"fat","platform":"linux-x64","mod_version":"1.2.3","fetch":"ok","write":"ok","mode":"external"}"#;
    assert!(HostCapability::parse(no_mode).is_err());
    assert!(HostCapability::parse(no_loader).is_err());
}

#[test]
fn a_java_report_missing_a_jar_field_is_refused() {
    let missing = r#"{"platform":"linux-x64","mod_version":"1.2.3","fetch":"ok","write":"ok","loader":"paper","mode":"external"}"#;
    assert!(HostCapability::parse(missing).is_err());
}
