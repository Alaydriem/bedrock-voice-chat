use bvc_server_lib::stream::quic::connection::PrefixedConnectionIdFormat;
use common::s2n_quic::provider::connection_id::{ConnectionInfo, Generator, Validator};

// The CID length and the Meridian instance-id prefix width, asserted as literals rather
// than read from the implementation, so a change to either has to be made deliberately here.
const DEFAULT_LEN: usize = 16;
const PREFIX_LEN: usize = 2;

#[test]
fn generate_produces_correct_length_and_prefix() {
    let instance_id: u16 = 42;
    let mut format = PrefixedConnectionIdFormat::new(instance_id);
    let remote_address = &common::s2n_quic_core::inet::SocketAddress::default();
    let connection_info = ConnectionInfo::new(remote_address);

    let id = format.generate(&connection_info);

    assert_eq!(id.len(), DEFAULT_LEN);
    let bytes = id.as_bytes();
    assert_eq!(&bytes[..PREFIX_LEN], &instance_id.to_be_bytes());
}

#[test]
fn generate_produces_unique_ids() {
    let mut format = PrefixedConnectionIdFormat::new(1);
    let remote_address = &common::s2n_quic_core::inet::SocketAddress::default();
    let connection_info = ConnectionInfo::new(remote_address);

    let id1 = format.generate(&connection_info);
    let id2 = format.generate(&connection_info);

    assert_ne!(id1.as_bytes(), id2.as_bytes());
}

#[test]
fn validate_accepts_correct_length() {
    let format = PrefixedConnectionIdFormat::new(1);
    let remote_address = &common::s2n_quic_core::inet::SocketAddress::default();
    let connection_info = ConnectionInfo::new(remote_address);

    assert_eq!(format.validate(&connection_info, &[0u8; 16]), Some(16));
    assert_eq!(format.validate(&connection_info, &[0u8; 20]), Some(16));
}

#[test]
fn validate_rejects_short_buffer() {
    let format = PrefixedConnectionIdFormat::new(1);
    let remote_address = &common::s2n_quic_core::inet::SocketAddress::default();
    let connection_info = ConnectionInfo::new(remote_address);

    assert_eq!(format.validate(&connection_info, &[0u8; 15]), None);
    assert_eq!(format.validate(&connection_info, &[0u8; 0]), None);
}
