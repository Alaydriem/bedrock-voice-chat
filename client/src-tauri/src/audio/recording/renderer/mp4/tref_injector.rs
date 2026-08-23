use crate::audio::recording::renderer::mp4::boxes::BoxWriter;

pub(super) struct TrefInjector;

impl TrefInjector {
    /// Create a tref box that references the timecode track
    pub(super) fn create_tref_to_timecode(timecode_track_id: u32) -> Vec<u8> {
        // Build tref box: tref contains a tmcd reference with the track ID
        // tref: size (4) + 'tref' (4) + tmcd reference
        // tmcd reference: size (4) + 'tmcd' (4) + track_id (4)
        let tmcd_size: u32 = 12; // 4 + 4 + 4
        let tref_size: u32 = 8 + tmcd_size;

        BoxWriter::new()
            .u32(tref_size)
            .fourcc(b"tref")
            .u32(tmcd_size)
            .fourcc(b"tmcd")
            .u32(timecode_track_id)
            .finish()
    }

    /// Inject tref box into the first trak box found in moov content
    /// Returns modified moov content (without moov header)
    pub(super) fn inject_tref_into_audio_trak(moov_content: &[u8], tref: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let mut pos = 0;
        let mut trak_modified = false;

        while pos + 8 <= moov_content.len() {
            let box_size = u32::from_be_bytes([
                moov_content[pos],
                moov_content[pos + 1],
                moov_content[pos + 2],
                moov_content[pos + 3],
            ]) as usize;

            let box_type = &moov_content[pos + 4..pos + 8];

            if box_size == 0 || pos + box_size > moov_content.len() {
                // Invalid box, copy rest and break
                result.extend_from_slice(&moov_content[pos..]);
                break;
            }

            if box_type == b"trak" && !trak_modified {
                // Found first trak (audio track) - inject tref after tkhd
                let trak_content = &moov_content[pos + 8..pos + box_size];
                let new_trak = TrefInjector::inject_tref_after_tkhd(trak_content, tref);

                // Write new trak with updated size
                let new_trak_size = (8 + new_trak.len()) as u32;
                result.extend_from_slice(&new_trak_size.to_be_bytes());
                result.extend_from_slice(b"trak");
                result.extend_from_slice(&new_trak);

                trak_modified = true;
            } else {
                // Copy box as-is
                result.extend_from_slice(&moov_content[pos..pos + box_size]);
            }

            pos += box_size;
        }

        result
    }

    /// Inject tref after tkhd in trak content
    pub(super) fn inject_tref_after_tkhd(trak_content: &[u8], tref: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let mut pos = 0;
        let mut tref_injected = false;

        while pos + 8 <= trak_content.len() {
            let box_size = u32::from_be_bytes([
                trak_content[pos],
                trak_content[pos + 1],
                trak_content[pos + 2],
                trak_content[pos + 3],
            ]) as usize;

            let box_type = &trak_content[pos + 4..pos + 8];

            if box_size == 0 || pos + box_size > trak_content.len() {
                result.extend_from_slice(&trak_content[pos..]);
                break;
            }

            // Copy current box
            result.extend_from_slice(&trak_content[pos..pos + box_size]);

            // Inject tref right after tkhd
            if box_type == b"tkhd" && !tref_injected {
                result.extend_from_slice(tref);
                tref_injected = true;
            }

            pos += box_size;
        }

        result
    }
}
