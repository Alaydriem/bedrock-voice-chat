use std::io::Cursor;

use ogg::writing::{PacketWriteEndInfo, PacketWriter};

/// A synthetic single-stream Ogg/Opus file.
///
/// Built rather than recorded. The parser derives a duration from two values only — the
/// `OpusHead` pre-skip and the final page's granule position — and a fixture that states
/// both can be checked against the duration it must produce. A recorded `.opus` file
/// states neither, and reading one from the server's own `assets/audio` directory ties
/// the test to whatever a developer last uploaded there.
pub struct OggFixture;

impl OggFixture {
    /// Arbitrary. Only its consistency across pages matters: the parser locks onto the
    /// serial of the first packet and ignores every other logical stream.
    pub const SERIAL: u32 = 0x0bad_c0de;

    /// Samples the decoder discards before the first audible one. Non-zero on purpose:
    /// it is subtracted from the final granule position, so a parser that skips the
    /// subtraction reports a longer track than this fixture contains.
    pub const PRE_SKIP: u16 = 312;

    /// 20 ms at 48 kHz, the frame size this project encodes at.
    pub const SAMPLES_PER_FRAME_20MS: u64 = 960;

    /// 120 ms at 48 kHz, the longest frame Opus permits. Reaches ten minutes of audio in
    /// a few thousand pages rather than tens of thousands.
    pub const SAMPLES_PER_FRAME_120MS: u64 = 5760;

    const SAMPLE_RATE: u64 = 48_000;

    /// An `OpusHead`, an `OpusTags` and `frames` audio packets, each on its own page and
    /// each carrying `samples_per_frame` samples.
    pub fn opus_stream(frames: u64, samples_per_frame: u64) -> Vec<u8> {
        let mut writer = PacketWriter::new(Cursor::new(Vec::new()));

        // Both headers take a page of their own, which is what the Opus-in-Ogg mapping
        // requires and what lets a reader identify the stream from its first page alone.
        writer
            .write_packet(
                Self::opus_head(),
                Self::SERIAL,
                PacketWriteEndInfo::EndPage,
                0,
            )
            .expect("write OpusHead");
        writer
            .write_packet(
                Self::opus_tags(),
                Self::SERIAL,
                PacketWriteEndInfo::EndPage,
                0,
            )
            .expect("write OpusTags");

        for frame in 1..=frames {
            let granule = Self::PRE_SKIP as u64 + frame * samples_per_frame;
            let end = if frame == frames {
                PacketWriteEndInfo::EndStream
            } else {
                PacketWriteEndInfo::EndPage
            };

            writer
                .write_packet(Self::audio_packet(), Self::SERIAL, end, granule)
                .expect("write audio packet");
        }

        writer.into_inner().into_inner()
    }

    /// The duration `opus_stream` with the same arguments must parse to.
    pub fn duration_ms(frames: u64, samples_per_frame: u64) -> u64 {
        (frames * samples_per_frame * 1000) / Self::SAMPLE_RATE
    }

    fn opus_head() -> Vec<u8> {
        let mut head = Vec::with_capacity(19);
        head.extend_from_slice(b"OpusHead");
        head.push(1);
        head.push(1);
        head.extend_from_slice(&Self::PRE_SKIP.to_le_bytes());
        head.extend_from_slice(&(Self::SAMPLE_RATE as u32).to_le_bytes());
        head.extend_from_slice(&0i16.to_le_bytes());
        head.push(0);
        head
    }

    fn opus_tags() -> Vec<u8> {
        const VENDOR: &[u8] = b"bvc-test";

        let mut tags = Vec::with_capacity(8 + 4 + VENDOR.len() + 4);
        tags.extend_from_slice(b"OpusTags");
        tags.extend_from_slice(&(VENDOR.len() as u32).to_le_bytes());
        tags.extend_from_slice(VENDOR);
        tags.extend_from_slice(&0u32.to_le_bytes());
        tags
    }

    // A well-formed TOC byte plus a two-byte payload. The parser never decodes a frame,
    // so the payload only has to be non-empty for the packet to be counted.
    fn audio_packet() -> Vec<u8> {
        vec![0xfc, 0x00, 0x00]
    }
}
