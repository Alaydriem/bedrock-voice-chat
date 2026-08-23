use bvc_client_lib::testkit::E2eAppData;

// The regression guard for the defect this type exists to remove.
//
// Every e2e client used to resolve one constant identifier, so all of them shared
// one app-data tree, and the harness recursively deleted that tree before every
// spawn. Under nextest — one process per test — those deletes ran concurrently
// with other tests' live clients, and a delete landing inside
// `tauri_plugin_http`'s create-then-open window killed the client with os error 3.
// The owning test then failed on a connect timeout, on a different test each run.
#[test]
fn a_namespace_is_never_the_bare_shared_identifier() {
    assert_ne!(
        E2eAppData::namespace("Alice"),
        E2eAppData::BASE_IDENTIFIER,
        "a client that resolves the bare identifier shares one tree with every \
         other concurrently running test"
    );
}

// Two clients in one scenario must not write each other's store.
#[test]
fn two_gamertags_get_two_namespaces() {
    assert_ne!(E2eAppData::namespace("Alice"), E2eAppData::namespace("Bob"));
}

// A client that is shut down and respawned inside one scenario must land back on
// its own tree rather than a fresh one.
#[test]
fn one_gamertag_is_stable_within_a_process() {
    assert_eq!(E2eAppData::namespace("Alice"), E2eAppData::namespace("Alice"));
}

// Reclamation resolves `<base>/BASE_IDENTIFIER/<process tag>`, so a namespace not
// rooted there leaks until someone deletes it by hand.
#[test]
fn a_namespace_carries_the_reclaimable_prefix() {
    assert!(
        E2eAppData::namespace("Alice").starts_with(E2eAppData::BASE_IDENTIFIER),
        "reclamation resolves this root"
    );
}

// A namespace is a relative path, not a name. Tauri joins it onto each base
// directory, so the base directories hold one entry for the whole suite rather
// than one per process per gamertag.
#[test]
fn a_namespace_nests_under_the_base_identifier() {
    let ns = E2eAppData::namespace("Alice");
    let components: Vec<&str> = ns.split('/').collect();

    assert_eq!(
        components.len(),
        3,
        "namespace {ns:?} should be base/process/gamertag"
    );
    assert_eq!(components[0], E2eAppData::BASE_IDENTIFIER);
}

// Reclamation removes one directory to collect every client of the calling
// process, which only holds if all of them nest under the same process directory.
#[test]
fn two_gamertags_share_one_process_directory() {
    let alice = E2eAppData::namespace("Alice");
    let bob = E2eAppData::namespace("Bob");

    let parent = |ns: &str| ns.rsplit_once('/').map(|(head, _)| head.to_string());

    assert_eq!(parent(&alice), parent(&bob));
    assert!(parent(&alice).is_some());
}

// Distinctness has to survive the gamertags the scenarios actually use, which
// include an empty one and one with characters that are not legal in a path. The
// gamertag contributes the leaf, so the separators a namespace carries must be
// only the two that nest it — never one a gamertag smuggled in.
#[test]
fn gamertags_that_are_not_path_safe_still_get_distinct_namespaces() {
    let awkward = ["", "a/b", "a\\b", "a:b", "..", "a b"];

    let mut seen = std::collections::HashSet::new();
    for tag in awkward {
        let ns = E2eAppData::namespace(tag);
        let leaf = ns.rsplit('/').next().expect("namespace has a leaf");

        assert!(
            !leaf.contains(['/', '\\', ':']),
            "leaf {leaf:?} for gamertag {tag:?} is not a single path component"
        );
        assert_eq!(
            ns.matches('/').count(),
            2,
            "namespace {ns:?} for gamertag {tag:?} has more components than base/process/gamertag"
        );
        assert!(
            seen.insert(ns.clone()),
            "gamertag {tag:?} collided onto an existing namespace {ns:?}"
        );
    }
}

// The child process resolves its own identifier from the environment, so the
// value the parent computes is the value the child must use.
#[test]
fn the_child_resolves_the_identifier_the_parent_sets() {
    let ns = E2eAppData::namespace("Alice");

    // SAFETY: single-threaded test process; nextest gives each test its own.
    unsafe { std::env::set_var(E2eAppData::ENV_VAR, &ns) };

    assert_eq!(E2eAppData::identifier(), ns);
}

// A harness run by hand, without the parent, must still stay off the real
// client's app-data rather than failing or falling back to production.
#[test]
fn a_child_with_no_environment_falls_back_to_the_base_identifier() {
    // SAFETY: single-threaded test process; nextest gives each test its own.
    unsafe { std::env::remove_var(E2eAppData::ENV_VAR) };

    assert_eq!(E2eAppData::identifier(), E2eAppData::BASE_IDENTIFIER);
}
