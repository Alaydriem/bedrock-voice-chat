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
