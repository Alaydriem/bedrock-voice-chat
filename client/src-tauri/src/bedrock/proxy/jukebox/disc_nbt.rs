use bytes::Bytes;

const BVC_DISC_NAME_PREFIX: &[u8] = b"BVC: ";
const UUID_LEN: usize = 36;

/// Scan an item's raw NBT extra blob for the BVC disc display name pattern
/// (`"BVC: <uuid>"`). The addon stores the audio id only as a script-side
/// dynamic property *and* duplicated into the disc's vanilla `nameTag`. Only
/// the nameTag survives onto the wire as part of the held-item NBT, so we
/// fish the UUID back out by substring match.
pub struct DiscNbt;

impl DiscNbt {
    pub fn extract_audio_id(extra: &Bytes) -> Option<String> {
        Self::extract_audio_id_bytes(extra.as_ref())
    }

    pub fn extract_audio_id_bytes(bytes: &[u8]) -> Option<String> {
        if bytes.len() < BVC_DISC_NAME_PREFIX.len() + UUID_LEN {
            return None;
        }
        let window_end = bytes.len() - UUID_LEN;
        for i in 0..=(window_end - BVC_DISC_NAME_PREFIX.len()) {
            if &bytes[i..i + BVC_DISC_NAME_PREFIX.len()] == BVC_DISC_NAME_PREFIX {
                let start = i + BVC_DISC_NAME_PREFIX.len();
                let uuid_slice = &bytes[start..start + UUID_LEN];
                if let Ok(uuid_str) = std::str::from_utf8(uuid_slice) {
                    if uuid::Uuid::parse_str(uuid_str).is_ok() {
                        return Some(uuid_str.to_string());
                    }
                }
            }
        }
        None
    }
}
