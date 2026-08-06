use bvc_server_lib::stream::quic::PacketIdentityStamp;
use common::structs::packet::{
    HealthCheckPacket, PacketSender, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};

fn inbound() -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::HealthCheck,
        data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
        // Not a server fan-out to one connection, so this envelope carries no sequence.
        ..Default::default()
    }
}

// The identity is the certificate CN verbatim, so whatever the client claimed is irrelevant —
// it no longer has a field to claim it in.
#[test]
fn the_stamped_identity_is_canonical() {
    let mut packet = inbound();
    PacketIdentityStamp::apply(&mut packet, "minecraft:Alaydriem", 7);

    let sender = packet.sender.expect("stamped");
    assert_eq!(sender.identity, "minecraft:Alaydriem");
    assert_eq!(sender.device, Some(7));
}

// Two devices for one player must stay distinguishable, which is what lets the mixer keep a
// separate sink per device while applying one set of gain settings.
#[test]
fn two_devices_share_an_identity_and_differ_by_device() {
    let mut first = inbound();
    let mut second = inbound();
    PacketIdentityStamp::apply(&mut first, "minecraft:Alaydriem", 1);
    PacketIdentityStamp::apply(&mut second, "minecraft:Alaydriem", 2);

    let a = first.sender.expect("stamped");
    let b = second.sender.expect("stamped");
    assert_eq!(a.identity, b.identity);
    assert_ne!(a.device, b.device);
}

// Stamping replaces whatever was there rather than reconciling with it. A packet that arrived
// carrying somebody else's sender must not keep it when this server fans it out under its own
// authority.
#[test]
fn stamping_overwrites_an_existing_sender() {
    let mut packet = inbound();
    packet.sender = Some(PacketSender::new("minecraft:Attacker".to_string(), 99));

    PacketIdentityStamp::apply(&mut packet, "minecraft:Alaydriem", 1);

    let sender = packet.sender.expect("stamped");
    assert_eq!(sender.identity, "minecraft:Alaydriem");
    assert_eq!(sender.device, Some(1));
}

// A real connection can hold device id 0, because s2n-quic counts internal connection ids from
// zero. Stamping it must produce a sender that is not mistaken for server-injected audio.
#[test]
fn the_first_connections_device_is_not_synthetic() {
    let mut packet = inbound();
    PacketIdentityStamp::apply(&mut packet, "minecraft:Alaydriem", 0);

    let sender = packet.sender.expect("stamped");
    assert!(!sender.is_synthetic());
}
