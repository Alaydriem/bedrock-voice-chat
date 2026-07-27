use bvc_server_lib::runtime::access_token::AccessTokenManager;
use tempfile::TempDir;

#[test]
fn configured_token_wins_and_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let mgr = AccessTokenManager::new(dir.path().to_str().unwrap());
    let token = mgr.resolve("configured-token").unwrap();
    assert_eq!(token, "configured-token");
    assert!(
        !dir.path().join("access_token").exists(),
        "a configured token must not be persisted"
    );
}

#[test]
fn generates_persists_and_reuses() {
    let dir = TempDir::new().unwrap();
    let mgr = AccessTokenManager::new(dir.path().to_str().unwrap());

    let first = mgr.resolve("").unwrap();
    assert_eq!(first.len(), 32);
    assert!(first.chars().all(|c| c.is_ascii_alphanumeric()));
    assert!(dir.path().join("access_token").exists());

    let second = mgr.resolve("").unwrap();
    assert_eq!(first, second, "second boot must reuse the persisted token");
}

#[test]
fn existing_file_is_trimmed_and_reused() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("access_token"), "pre-existing-token\n").unwrap();
    let mgr = AccessTokenManager::new(dir.path().to_str().unwrap());
    let token = mgr.resolve("  ").unwrap();
    assert_eq!(token, "pre-existing-token");
}

#[test]
fn distinct_paths_generate_distinct_tokens() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    let token_a = AccessTokenManager::new(a.path().to_str().unwrap())
        .resolve("")
        .unwrap();
    let token_b = AccessTokenManager::new(b.path().to_str().unwrap())
        .resolve("")
        .unwrap();
    assert_ne!(token_a, token_b);
}
