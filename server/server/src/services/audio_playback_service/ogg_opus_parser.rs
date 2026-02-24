use std::io::Cursor;

use ogg::reading::PacketReader;

/// Parses Ogg/Opus files to extract duration, frame count, and raw Opus frames.
pub(crate) struct OggOpusParser;

impl OggOpusParser {
    /// Parse an Ogg/Opus file in memory and return (duration_ms, frame_count).
    pub fn parse_duration(data: &[u8]) -> Result<(u64, usize), String> {
        let cursor = Cursor::new(data);
        let mut reader = PacketReader::new(cursor);

        let mut frame_count = 0usize;
        let mut last_granule: u64 = 0;
        let mut pre_skip: u16 = 0;
        let mut packet_index = 0u32;
        let mut target_serial: Option<u32> = None;

        loop {
            match reader.read_packet() {
                Ok(Some(packet)) => {
                    if packet_index == 0 {
                        target_serial = Some(packet.stream_serial());
                        if packet.data.len() >= 12 && &packet.data[0..8] == b"OpusHead" {
                            pre_skip = u16::from_le_bytes([packet.data[10], packet.data[11]]);
                        }
                    } else if packet_index >= 2 {
                        if let Some(serial) = target_serial {
                            if packet.stream_serial() == serial && !packet.data.is_empty() {
                                frame_count += 1;
                                if packet.absgp_page() > 0 {
                                    last_granule = packet.absgp_page();
                                }
                            }
                        }
                    }
                    packet_index += 1;
                }
                Ok(None) => break,
                Err(e) => return Err(format!("Ogg read error: {}", e)),
            }
        }

        let sample_count = last_granule.saturating_sub(pre_skip as u64);
        let duration_ms = (sample_count * 1000) / 48000;

        Ok((duration_ms, frame_count))
    }

    /// Parse an Ogg/Opus file from disk and return the raw Opus frames and duration in milliseconds.
    pub fn parse_frames(file_path: &str) -> Result<(Vec<Vec<u8>>, u64), String> {
        let file_data =
            std::fs::read(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

        let cursor = Cursor::new(file_data);
        let mut reader = PacketReader::new(cursor);

        let mut frames: Vec<Vec<u8>> = Vec::new();
        let mut target_serial: Option<u32> = None;
        let mut last_granule: u64 = 0;
        let mut pre_skip: u16 = 0;
        let mut packet_index = 0u32;

        loop {
            match reader.read_packet() {
                Ok(Some(packet)) => {
                    if packet_index == 0 {
                        target_serial = Some(packet.stream_serial());
                        if packet.data.len() >= 12 && &packet.data[0..8] == b"OpusHead" {
                            pre_skip = u16::from_le_bytes([packet.data[10], packet.data[11]]);
                        }
                    } else if packet_index == 1 {
                        // OpusTags — skip
                    } else {
                        if let Some(serial) = target_serial {
                            if packet.stream_serial() == serial && !packet.data.is_empty() {
                                frames.push(packet.data.to_vec());
                                if packet.absgp_page() > 0 {
                                    last_granule = packet.absgp_page();
                                }
                            }
                        }
                    }
                    packet_index += 1;
                }
                Ok(None) => break,
                Err(e) => return Err(format!("Ogg read error: {}", e)),
            }
        }

        let sample_count = last_granule.saturating_sub(pre_skip as u64);
        let duration_ms = (sample_count * 1000) / 48000;

        Ok((frames, duration_ms))
    }
}
