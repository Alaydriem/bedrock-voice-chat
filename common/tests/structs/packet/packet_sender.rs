use common::structs::packet::PacketSender;

// s2n-quic hands out internal connection ids from a counter starting at zero, so the first
// connection after every restart holds id 0. A sentinel value for "not a connection" would
// therefore steal a real player's identity once per server start — the reason absence is
// modelled by `None` rather than by a reserved number.
#[test]
fn device_zero_is_a_real_connection_not_a_synthetic_sender() {
    let first_connection = PacketSender::new("minecraft:Alaydriem".to_string(), 0);
    assert!(!first_connection.is_synthetic());
    assert_eq!(first_connection.device, Some(0));
}

#[test]
fn an_injected_sender_has_no_device() {
    let jukebox = PacketSender::synthetic("jukebox-abc");
    assert!(jukebox.is_synthetic());
    assert_eq!(jukebox.device, None);
}

// Two devices for one player share an identity and differ by device, which is what lets the
// mixer hold a separate sink per device while applying one set of gain settings.
#[test]
fn two_devices_of_one_player_share_an_identity() {
    let phone = PacketSender::new("minecraft:Alaydriem".to_string(), 1);
    let desktop = PacketSender::new("minecraft:Alaydriem".to_string(), 2);
    assert_eq!(phone.identity, desktop.identity);
    assert_ne!(phone.device, desktop.device);
}
