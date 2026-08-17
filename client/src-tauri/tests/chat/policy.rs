use bvc_client_lib::chat::ChatPolicy;

// Permissive until a server says otherwise: the same answer an unasked server gives, and the
// state every client is in before it connects to anything.
#[test]
fn a_fresh_policy_permits_chat() {
    let policy = ChatPolicy::new_shared();

    assert!(policy.is_enabled());
}

#[test]
fn a_policy_reports_what_it_was_told() {
    let policy = ChatPolicy::new_shared();

    policy.set_enabled(false);
    assert!(!policy.is_enabled());

    policy.set_enabled(true);
    assert!(policy.is_enabled());
}
