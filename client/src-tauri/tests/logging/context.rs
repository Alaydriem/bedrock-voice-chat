use bvc_client_lib::logging::LogContext;

#[test]
fn keys_are_absent_before_setup_completes() {
    let context = LogContext::new();
    let keys = context.snapshot();

    assert!(keys.platform_id.is_none());
    assert!(keys.install_id.is_none());
    assert!(keys.session_id.is_none());
}

#[test]
fn keys_are_readable_once_set() {
    let context = LogContext::new();
    context.set(
        "platform".to_string(),
        "install".to_string(),
        "session".to_string(),
    );

    let keys = context.snapshot();
    assert_eq!(keys.platform_id.as_deref(), Some("platform"));
    assert_eq!(keys.install_id.as_deref(), Some("install"));
    assert_eq!(keys.session_id.as_deref(), Some("session"));
}
