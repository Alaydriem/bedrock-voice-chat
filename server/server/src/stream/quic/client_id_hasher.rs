use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(crate) struct ClientIdHasher;

impl ClientIdHasher {
    pub fn hash(client_id: &[u8]) -> String {
        let mut hasher = DefaultHasher::new();
        client_id.hash(&mut hasher);
        format!("{:x}", hasher.finish() & 0xFFFF)
    }
}
