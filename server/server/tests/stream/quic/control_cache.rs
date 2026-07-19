use bvc_server_lib::stream::quic::{CacheManager, CacheTrait};
use common::structs::control::{PlayerPreference, PreferenceKey, QueryState};

fn state(id: &str, muted: bool) -> QueryState {
    QueryState {
        id: id.into(),
        muted,
        deafened: false,
        recording: false,
        current_group: None,
    }
}

#[tokio::test]
async fn query_state_put_get_latest() {
    let cm = CacheManager::new();
    cm.player_state().set("Alice".into(), state("Alice", true)).await;
    assert_eq!(cm.player_state().get(&"Alice".into()).await.unwrap().muted, true);

    cm.player_state()
        .set("Alice".into(), state("Alice", false))
        .await;
    assert_eq!(
        cm.player_state().get(&"Alice".into()).await.unwrap().muted,
        false
    );

    assert!(cm.player_state().get(&"Nobody".into()).await.is_none());
}

fn pref(owner: &str, target: &str, vol: f32) -> PlayerPreference {
    PlayerPreference {
        owner: owner.into(),
        target: target.into(),
        volume: vol,
        muted: false,
    }
}

#[tokio::test]
async fn scoped_preferences_returns_only_requested_and_owner_isolated() {
    let cm = CacheManager::new();
    let p = cm.preferences();
    p.set(PreferenceKey::new("Alice", "Steve"), pref("Alice", "Steve", 0.7))
        .await;
    p.set(PreferenceKey::new("Alice", "Bob"), pref("Alice", "Bob", 0.2))
        .await;
    p.set(PreferenceKey::new("Zed", "Steve"), pref("Zed", "Steve", 0.9))
        .await;

    let scoped = p
        .get_scoped("Alice", &["Steve".to_string(), "Nobody".to_string()])
        .await;
    assert_eq!(scoped.len(), 1, "only Alice->Steve, not the miss or Zed's row");
    assert_eq!(scoped[0].target, "Steve");
    assert_eq!(scoped[0].volume, 0.7);

    assert_eq!(
        p.get_scoped("Zed", &["Bob".to_string()]).await.len(),
        0,
        "owner isolation: Zed has no Bob pref"
    );
}

#[tokio::test]
async fn set_sanitizes_out_of_range_volume() {
    let cm = CacheManager::new();
    let p = cm.preferences();
    p.set(PreferenceKey::new("Alice", "Loud"), pref("Alice", "Loud", 9000.0))
        .await;
    p.set(PreferenceKey::new("Alice", "Nan"), pref("Alice", "Nan", f32::NAN))
        .await;

    let loud = p.get(&PreferenceKey::new("Alice", "Loud")).await.unwrap();
    assert_eq!(loud.volume, 2.0, "clamped to the max");
    let nan = p.get(&PreferenceKey::new("Alice", "Nan")).await.unwrap();
    assert_eq!(nan.volume, 0.0, "non-finite mapped to 0");
}
