use bvc_server_lib::config::{Registry, Server};

// Baked in at build time and readable nowhere else. `sanitize` is the seam: the value
// `option_env!` returns is fixed when this test binary is compiled, so the decisions
// made about it are what can be exercised, not the value itself.

// A build produced without `BVC_REGISTRY_PEERLINK` has no registry, and has to say so
// rather than dial an empty string.
#[test]
fn an_absent_bake_is_no_registry() {
    assert_eq!(Registry::sanitize(None), None);
}

// `BVC_REGISTRY_PEERLINK=` in `.env.local` reaches `option_env!` as `Some("")`, not as
// `None`. Carrying that forward fails at the dial with a parse error instead of at the
// point where the honest answer is "this build has no registry".
#[test]
fn a_blank_bake_is_no_registry() {
    assert_eq!(Registry::sanitize(Some("")), None);
    assert_eq!(Registry::sanitize(Some("   ")), None);
    assert_eq!(Registry::sanitize(Some("\n\t")), None);
}

// A value written into a file usually arrives with the newline that ended the line.
#[test]
fn a_baked_value_is_trimmed() {
    assert_eq!(
        Registry::sanitize(Some("  bvcpeerlocal\n")),
        Some("bvcpeerlocal".to_string())
    );
}

// Whatever this build baked, the accessor never yields a blank string. This is the
// invariant that holds no matter which environment compiled the binary, and it is the
// one the dial depends on.
#[test]
fn the_baked_registry_is_never_blank() {
    if let Some(peerlink) = Registry::peerlink() {
        assert!(!peerlink.trim().is_empty());
        assert_eq!(peerlink, peerlink.trim());
    }
}

// The property the bake exists for: a shipped binary cannot be redirected at a
// registry its builder did not choose. This fails the moment anyone reintroduces a
// `std::env::var` read, which is the regression that would quietly restore a runtime
// override.
#[test]
fn a_runtime_environment_variable_does_not_change_the_bake() {
    let baked = Registry::peerlink();

    // SAFETY: single-threaded within this test, and nothing in the crate reads this
    // variable at runtime any more — which is precisely what is being asserted.
    unsafe {
        std::env::set_var("BVC_REGISTRY_PEERLINK", "bvcpeerruntimeoverride");
    }
    let after = Registry::peerlink();
    unsafe {
        std::env::remove_var("BVC_REGISTRY_PEERLINK");
    }

    assert_eq!(after, baked);
    assert_ne!(after, Some("bvcpeerruntimeoverride".to_string()));
}

// The registry serves enrollment and address observation, which are independent
// features with different audiences. Reading it must not require an enrollment token,
// because a server that will never enroll still asks for its own address.
#[test]
fn the_registry_is_readable_without_an_enrollment_token() {
    let config = Server::default();

    assert_eq!(config.enrollment.token(), None);
    // Does not panic and does not depend on any config: it is not config.
    let _ = Registry::peerlink();
}
