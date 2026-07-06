use bvc_server_lib::stream::quic::connection_registry::ConnectionRegistry;
use tokio::sync::mpsc;

#[test]
fn reaper_purges_absent_channel_membership_after_grace() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.register(vec![1], "Alice".to_string(), tx.clone());
    reg.update_player_channel("minecraft:Alice".to_string(), "chan1".to_string());
    reg.update_player_channel("minecraft:Ghost".to_string(), "chan2".to_string());
    assert_eq!(reg.channel_membership_count(), 2);

    // Ghost has no live connection; the grace is 2 sweeps, so one sweep keeps it.
    reg.reap_stale_channels();
    assert_eq!(reg.channel_membership_count(), 2);

    // Second sweep past grace purges Ghost; Alice (live) is never reaped.
    reg.reap_stale_channels();
    assert_eq!(reg.channel_membership_count(), 1);
}

#[test]
fn reaper_resets_grace_when_player_reconnects() {
    let reg = ConnectionRegistry::new();
    let (tx, _rx) = mpsc::channel(4);
    reg.update_player_channel("minecraft:Bob".to_string(), "chan1".to_string());

    reg.reap_stale_channels(); // sweep 1 while Bob is absent
    reg.register(vec![9], "Bob".to_string(), tx.clone()); // Bob reconnects
    reg.reap_stale_channels(); // Bob live now -> grace reset, not purged

    assert_eq!(reg.channel_membership_count(), 1);
}
