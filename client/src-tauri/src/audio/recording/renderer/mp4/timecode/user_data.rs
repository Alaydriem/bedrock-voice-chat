//! User data box (udta) with session metadata
//!
//! Contains custom metadata atoms for session identification and synchronization.

use crate::audio::recording::renderer::mp4::boxes::BoxWriter;
use crate::audio::recording::renderer::mp4::constants::{METADATA_TYPE_UINT, METADATA_TYPE_UTF8};
use crate::audio::recording::renderer::stream::opus::OpusStreamInfo;

/// Session metadata for the user data box
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    /// Unique session identifier
    pub session_id: String,
    /// Unix timestamp in milliseconds when the session started
    pub start_timestamp: u64,
    /// Name of the player this audio belongs to
    pub player_name: String,
    /// Duration of the recording in milliseconds (if known)
    pub duration_ms: Option<u64>,
}

impl SessionMetadata {
    /// Create session metadata from OpusStreamInfo
    pub fn from_stream_info(info: &OpusStreamInfo, duration_ms: Option<u64>) -> Self {
        Self {
            session_id: info.session_info.session_id.clone(),
            start_timestamp: info.session_info.start_timestamp,
            player_name: info.session_info.player_name.clone(),
            duration_ms,
        }
    }
}

/// User data box containing session metadata
///
/// Structure:
/// ```text
/// udta
/// └── meta
///     ├── hdlr (mdir handler)
///     └── ilst
///         ├── seid (session ID)
///         ├── stts (start timestamp)
///         ├── plyr (player name)
///         └── dura (duration, optional)
/// ```
#[derive(Debug, Clone)]
pub struct UserDataBox {
    metadata: SessionMetadata,
}

impl UserDataBox {
    /// Create a user data box from OpusStreamInfo
    pub fn from_stream_info(info: &OpusStreamInfo, duration_ms: Option<u64>) -> Self {
        Self {
            metadata: SessionMetadata::from_stream_info(info, duration_ms),
        }
    }

    /// Create a user data box from SessionMetadata
    pub fn from_metadata(metadata: SessionMetadata) -> Self {
        Self { metadata }
    }

    /// Get the underlying metadata
    pub fn metadata(&self) -> &SessionMetadata {
        &self.metadata
    }

    /// Serialize to bytes
    ///
    /// Returns the complete udta box ready to be appended to the moov box.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut udta = BoxWriter::new();

        // Build meta box
        let meta = self.build_meta();
        udta = udta.write_box(b"meta", &meta);

        // Wrap in udta
        BoxWriter::new().write_box(b"udta", udta.as_bytes()).finish()
    }

    /// Build the meta box
    fn build_meta(&self) -> Vec<u8> {
        let mut meta = BoxWriter::new();

        // Version 0, flags 0
        meta = meta.u32(0);

        // hdlr for meta
        let hdlr = self.build_meta_hdlr();
        meta = meta.write_box(b"hdlr", &hdlr);

        // ilst (item list) with custom data
        let ilst = self.build_ilst();
        meta = meta.write_box(b"ilst", &ilst);

        meta.finish()
    }

    /// Build the metadata handler box
    fn build_meta_hdlr(&self) -> Vec<u8> {
        BoxWriter::new()
            .u32(0) // version/flags
            .u32(0) // pre-defined
            .fourcc(b"mdir") // handler type (metadata)
            .u32(0) // reserved
            .u32(0)
            .u32(0)
            .bytes(b"\0") // empty name
            .finish()
    }

    /// Build the item list box
    fn build_ilst(&self) -> Vec<u8> {
        let mut ilst = BoxWriter::new();

        // Session ID
        let seid = Self::build_string_atom(&self.metadata.session_id);
        ilst = ilst.write_box(b"seid", &seid);

        // Start timestamp
        let stts = Self::build_u64_atom(self.metadata.start_timestamp);
        ilst = ilst.write_box(b"stts", &stts);

        // Player name
        let plyr = Self::build_string_atom(&self.metadata.player_name);
        ilst = ilst.write_box(b"plyr", &plyr);

        // Duration (optional)
        if let Some(duration) = self.metadata.duration_ms {
            let dura = Self::build_u64_atom(duration);
            ilst = ilst.write_box(b"dura", &dura);
        }

        ilst.finish()
    }

    /// Build a custom string atom with 'data' box wrapper
    fn build_string_atom(value: &str) -> Vec<u8> {
        let data = BoxWriter::new()
            .u32(METADATA_TYPE_UTF8) // type: UTF-8
            .u32(0) // locale
            .bytes(value.as_bytes())
            .finish();

        BoxWriter::new().write_box(b"data", &data).finish()
    }

    /// Build a custom u64 atom with 'data' box wrapper
    fn build_u64_atom(value: u64) -> Vec<u8> {
        let data = BoxWriter::new()
            .u32(METADATA_TYPE_UINT) // type: unsigned int
            .u32(0) // locale
            .u64(value)
            .finish();

        BoxWriter::new().write_box(b"data", &data).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_data_box_structure() {
        let metadata = SessionMetadata {
            session_id: "test-session".to_string(),
            start_timestamp: 1705329045500,
            player_name: "TestPlayer".to_string(),
            duration_ms: Some(60000),
        };

        let udta = UserDataBox::from_metadata(metadata);
        let bytes = udta.to_bytes();

        // Verify it starts with a valid box header
        assert!(bytes.len() >= 8);
        // Check fourcc is 'udta'
        assert_eq!(&bytes[4..8], b"udta");
    }

    #[test]
    fn test_user_data_box_without_duration() {
        let metadata = SessionMetadata {
            session_id: "test-session".to_string(),
            start_timestamp: 1705329045500,
            player_name: "TestPlayer".to_string(),
            duration_ms: None,
        };

        let udta = UserDataBox::from_metadata(metadata);
        let bytes = udta.to_bytes();

        // Should still produce valid output
        assert!(bytes.len() >= 8);
        assert_eq!(&bytes[4..8], b"udta");
    }

    #[test]
    fn test_string_atom_format() {
        let atom = UserDataBox::build_string_atom("test");

        // Should be: data box header (8) + type (4) + locale (4) + "test" (4)
        // Total: 20 bytes
        assert_eq!(atom.len(), 20);

        // Verify data box header
        let size = u32::from_be_bytes([atom[0], atom[1], atom[2], atom[3]]);
        assert_eq!(size, 20);
        assert_eq!(&atom[4..8], b"data");

        // Verify type is UTF-8 (1)
        let data_type = u32::from_be_bytes([atom[8], atom[9], atom[10], atom[11]]);
        assert_eq!(data_type, METADATA_TYPE_UTF8);
    }

    #[test]
    fn test_u64_atom_format() {
        let atom = UserDataBox::build_u64_atom(0x1234567890ABCDEF);

        // Should be: data box header (8) + type (4) + locale (4) + value (8)
        // Total: 24 bytes
        assert_eq!(atom.len(), 24);

        // Verify data box header
        let size = u32::from_be_bytes([atom[0], atom[1], atom[2], atom[3]]);
        assert_eq!(size, 24);
        assert_eq!(&atom[4..8], b"data");

        // Verify type is unsigned int (0x15)
        let data_type = u32::from_be_bytes([atom[8], atom[9], atom[10], atom[11]]);
        assert_eq!(data_type, METADATA_TYPE_UINT);
    }
}
