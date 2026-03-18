use common::s2n_quic::provider::connection_id::{ConnectionInfo, Generator, LocalId, Validator};
use rand::Rng;

const PREFIX_LEN: usize = 2;
const DEFAULT_LEN: usize = 16;

pub struct PrefixedConnectionIdFormat {
    instance_id: u16,
    len: usize,
}

impl PrefixedConnectionIdFormat {
    pub fn new(instance_id: u16) -> Self {
        Self {
            instance_id,
            len: DEFAULT_LEN,
        }
    }
}

impl Generator for PrefixedConnectionIdFormat {
    fn generate(&mut self, _connection_info: &ConnectionInfo) -> LocalId {
        let mut buf = [0u8; 20];
        let id = &mut buf[..self.len];
        id[..PREFIX_LEN].copy_from_slice(&self.instance_id.to_be_bytes());
        rand::rng().fill_bytes(&mut id[PREFIX_LEN..]);
        LocalId::try_from_bytes(id).expect("length already validated in constructor")
    }
}

impl Validator for PrefixedConnectionIdFormat {
    fn validate(&self, _connection_info: &ConnectionInfo, buffer: &[u8]) -> Option<usize> {
        if buffer.len() >= self.len {
            Some(self.len)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
