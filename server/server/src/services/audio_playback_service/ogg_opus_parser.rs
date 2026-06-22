use std::io::{Cursor, Read as IoRead, Seek};

use ogg::reading::PacketReader;

use super::parse_result::ParseResult;

pub(crate) struct OggOpusParser;

impl OggOpusParser {
    pub fn parse_duration(data: &[u8]) -> Result<(u64, usize), String> {
        let cursor = Cursor::new(data);
        let result = Self::parse_ogg_packets(cursor, false)?;
        Ok((result.duration_ms, result.frame_count))
    }

    pub fn parse_frames(file_path: &str) -> Result<(Vec<Vec<u8>>, u64), String> {
        let data = std::fs::read(file_path).map_err(|e| format!("Failed to read file: {}", e))?;
        Self::parse_frames_bytes(&data)
    }

    pub fn parse_frames_bytes(data: &[u8]) -> Result<(Vec<Vec<u8>>, u64), String> {
        let cursor = Cursor::new(data);
        let result = Self::parse_ogg_packets(cursor, true)?;
        Ok((result.frames, result.duration_ms))
    }

    fn parse_ogg_packets<R: IoRead + Seek>(
        source: R,
        collect_frames: bool,
    ) -> Result<ParseResult, String> {
        let mut reader = PacketReader::new(source);

        let mut frames: Vec<Vec<u8>> = Vec::new();
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
                            if packet.data[8] != 1 {
                                return Err(format!(
                                    "Unsupported OpusHead version: {}",
                                    packet.data[8]
                                ));
                            }
                            pre_skip = u16::from_le_bytes([packet.data[10], packet.data[11]]);
                        }
                    } else if packet_index >= 2 {
                        if let Some(serial) = target_serial {
                            if packet.stream_serial() == serial && !packet.data.is_empty() {
                                frame_count += 1;
                                if packet.absgp_page() > 0 {
                                    last_granule = packet.absgp_page();
                                }
                                if collect_frames {
                                    frames.push(packet.data.to_vec());
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

        Ok(ParseResult {
            frames,
            frame_count,
            duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path() -> &'static str {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/audio/019d1701-6f1c-7661-ac44-4302ad6ba2f9.opus"
        )
    }

    #[test]
    fn parse_frames_bytes_matches_parse_frames() {
        let path = fixture_path();
        let (frames_path, duration_path) =
            OggOpusParser::parse_frames(path).expect("parse_frames failed");

        let data = std::fs::read(path).expect("fixture read failed");
        let (frames_bytes, duration_bytes) =
            OggOpusParser::parse_frames_bytes(&data).expect("parse_frames_bytes failed");

        assert_eq!(frames_path.len(), frames_bytes.len(), "frame counts differ");
        assert_eq!(duration_path, duration_bytes, "durations differ");
        assert_eq!(frames_path, frames_bytes, "frame data differs");
    }
}
