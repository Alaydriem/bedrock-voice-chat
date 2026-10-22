/// A frame that decrypted and authenticated, with the MLS leaf whose key sealed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedFrame {
    pub sender_leaf: u32,
    pub opus: Vec<u8>,
}
