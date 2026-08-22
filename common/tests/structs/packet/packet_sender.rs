use common::structs::packet::PacketSender;
use common::{Game, PlayerIdentity};

fn identity(gamertag: &str) -> PlayerIdentity {
    Game::Minecraft.membership_key(gamertag)
}

// s2n-quic hands out internal connection ids from a counter starting at zero, so the first
// connection after every restart holds id 0. A sentinel value for "not a connection" would
// therefore steal a real player's identity once per server start — the reason absence is
// modelled by `None` rather than by a reserved number.
#[test]
fn device_zero_is_a_real_connection() {
    let first = PacketSender::player(identity("Alaydriem"), 0);
    assert_eq!(first.device(), Some(0));
    assert!(first.identity().is_some());
}

// A server-injected sender is not a player and has no connection, so neither an identity
// nor a device is available to attribute it to one.
#[test]
fn a_service_sender_has_no_player_and_no_device() {
    let jukebox = PacketSender::for_service("jukebox-abc");
    assert!(jukebox.identity().is_none());
    assert_eq!(jukebox.device(), None);
    assert_eq!(jukebox.service(), Some("jukebox-abc"));
}

// A relayed peer is a real player this server holds no connection for, which is a
// different absence from a service and must not be mistaken for one.
#[test]
fn a_relayed_player_has_an_identity_but_no_device() {
    let peer = PacketSender::relayed(identity("FarAway"));
    assert_eq!(peer.identity(), Some(&identity("FarAway")));
    assert_eq!(peer.device(), None);
    assert!(peer.service().is_none());
}

// The reduced form the audio egress sends carries a device and nothing else. A reader that
// wanted an identity here must resolve it rather than read it, which is what makes the
// absence explicit instead of silent.
#[test]
fn the_reduced_form_carries_only_a_device() {
    let reduced = PacketSender::Device(7);
    assert!(reduced.identity().is_none());
    assert_eq!(reduced.device(), Some(7));
    assert!(reduced.service().is_none());
}

// Two devices for one player share an identity and differ by device, which is what lets the
// mixer hold a separate sink per device while applying one set of gain settings.
#[test]
fn two_devices_of_one_player_share_an_identity() {
    let phone = PacketSender::player(identity("Alaydriem"), 1);
    let desktop = PacketSender::player(identity("Alaydriem"), 2);
    assert_eq!(phone.identity(), desktop.identity());
    assert_ne!(phone.device(), desktop.device());
}

// The reduced form is what the whole change is for. If it is not materially smaller than
// the full one, the elision on the egress buys nothing.
#[test]
fn the_reduced_form_is_smaller_on_the_wire() {
    let full = postcard::to_stdvec(&PacketSender::player(identity("Alaydriem"), 7)).unwrap();
    let reduced = postcard::to_stdvec(&PacketSender::Device(7)).unwrap();
    assert!(
        reduced.len() * 4 < full.len(),
        "reduced {} should be far smaller than full {}",
        reduced.len(),
        full.len()
    );
}
