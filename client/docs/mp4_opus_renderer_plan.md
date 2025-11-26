# MP4 Opus Renderer Implementation Plan

## Overview

Create an MP4 renderer that muxes raw Opus packets directly (no transcoding) into an MP4/M4A container with professional timecode support for NLE compatibility (Final Cut Pro, DaVinci Resolve, Premiere Pro, etc.).

### Goals
- **Lossless Opus passthrough** - No decode/re-encode, preserving original quality
- **~10x compression** vs BWAV (Opus vs uncompressed PCM)
- **Professional timecode** - tmcd track + nmhd header + udta metadata
- **Consistent patterns** - Follow existing `BwavRenderer` architecture

---

## Library Selection

### Rejected: `alfg/mp4-rust`
The original library referenced does **not** support Opus codec. It only supports:
- AAC audio (AacConfig)
- H.264/H.265/VP9 video

### Selected: `shiguredo-mp4`
GitHub: https://github.com/shiguredo/mp4-rust
Crate: `shiguredo-mp4`

**Features:**
- Native Opus support via `OpusBox` and `DopsBox`
- Sans I/O architecture (we control file writing)
- `UnknownBox` allows custom box construction for tmcd/nmhd/udta
- Zero external dependencies, `no_std` compatible

**Key Types:**
```rust
// Opus sample entry (stsd child)
OpusBox {
    audio: AudioSampleEntryFields,
    dops_box: DopsBox,
    unknown_boxes: Vec<UnknownBox>,
}

// Opus-specific configuration (per RFC 7845 / Opus-in-ISOBMFF)
DopsBox {
    output_channel_count: u8,   // 1=mono, 2=stereo
    pre_skip: u16,              // Encoder delay in samples
    input_sample_rate: u32,     // Original sample rate
    output_gain: i16,           // Gain adjustment in dB Q7.8
}

// Muxer sample registration
Sample {
    track_kind: TrackKind,
    sample_entry: Option<SampleEntry>,  // First sample only
    keyframe: bool,
    timescale: NonZeroU32,
    duration: u32,
    data_offset: u64,
    data_size: usize,
}
```

---

## Architecture

### Data Flow Comparison

**Current BWAV Flow (lossy):**
```
WAL (Opus) → WalAudioReader → decode → PcmStream (f32) → BwavRenderer → .wav
```

**New MP4 Flow (lossless):**
```
WAL (Opus) → WalAudioReader → OpusPacketStream (raw) → Mp4Renderer → .m4a
```

### New Files

| File | Purpose |
|------|---------|
| `renderer/opus_stream.rs` | `OpusPacketStream` iterator + `SilenceEncoder` |
| `renderer/mp4.rs` | `Mp4Renderer` implementation |
| `renderer/timecode.rs` | Timecode track box construction |

### Modified Files

| File | Changes |
|------|---------|
| `renderer/mod.rs` | Add `AudioFormat::Mp4Opus`, exports |
| `Cargo.toml` | Add `shiguredo-mp4` dependency |

---

## Implementation Details

### 1. OpusPacketStream (`opus_stream.rs`)

New iterator that yields raw Opus packets without decoding:

```rust
/// Chunk of raw Opus data for lossless muxing
#[derive(Debug)]
pub enum OpusChunk {
    /// Raw Opus packet from WAL
    Packet {
        data: Vec<u8>,
        duration_samples: u32,  // e.g., 960 for 20ms @ 48kHz
    },
    /// Silence gap requiring encoded silence packets
    Silence {
        duration_samples: u32,
    },
}

/// Stream metadata extracted from first packet
#[derive(Debug, Clone)]
pub struct OpusStreamInfo {
    pub sample_rate: u32,               // Always 48000 for Opus
    pub channels: u16,                  // 1 or 2
    pub first_packet_timestamp_ms: u64, // For timecode calculation
    pub session_info: SessionInfo,      // From session.json
}

/// Iterator over raw Opus packets from WAL files
pub struct OpusPacketStream {
    reader: WalAudioReader,
    info: Option<OpusStreamInfo>,
    pending_silence: Option<u32>,
    finished: bool,
}

impl OpusPacketStream {
    /// Create stream from WAL session
    pub fn new(session_path: &Path, player_name: &str) -> Result<Self, anyhow::Error>;

    /// Get stream metadata (None if no packets)
    pub fn info(&self) -> Option<&OpusStreamInfo>;
}

impl Iterator for OpusPacketStream {
    type Item = Result<OpusChunk, anyhow::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // 1. Check for pending silence from gap calculation
        // 2. Calculate silence before next packet (reuse WalAudioReader logic)
        // 3. Yield raw packet from WalAudioReader::next_raw_entry()
    }
}
```

**Key Differences from PcmStream:**
- Uses `next_raw_entry()` instead of `next_frame()` (no Opus decoding)
- Yields `Vec<u8>` raw packets, not `Vec<f32>` PCM
- Silence represented as sample count (renderer encodes it)

### 2. Silence Encoder

Encode actual silence (zeros) as Opus packets for gap filling:

```rust
/// Generates Opus-encoded silence packets
pub struct SilenceEncoder {
    encoder: opus::Encoder,
    sample_rate: u32,
    channels: u16,
    frame_size: usize,  // 960 samples for 20ms @ 48kHz
}

impl SilenceEncoder {
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self, anyhow::Error> {
        let channels_enum = if channels == 1 {
            opus::Channels::Mono
        } else {
            opus::Channels::Stereo
        };

        let mut encoder = opus::Encoder::new(sample_rate, channels_enum, opus::Application::Audio)?;

        // Disable DTX to ensure consistent packet output
        encoder.set_dtx(false)?;

        // Match typical voice bitrate
        encoder.set_bitrate(opus::Bitrate::Bits(64000))?;

        Ok(Self {
            encoder,
            sample_rate,
            channels,
            frame_size: (sample_rate as usize * 20) / 1000, // 20ms frames
        })
    }

    /// Encode silence for given duration
    /// Returns Vec of (packet_data, duration_samples)
    pub fn encode_silence(&mut self, total_samples: u32) -> Result<Vec<(Vec<u8>, u32)>, anyhow::Error> {
        let mut packets = Vec::new();
        let mut remaining = total_samples as usize;

        // Zero buffer for encoding silence
        let silence: Vec<f32> = vec![0.0; self.frame_size * self.channels as usize];
        let mut output = vec![0u8; 4000]; // Max Opus packet size

        while remaining > 0 {
            let frame_samples = remaining.min(self.frame_size);

            let encoded_len = self.encoder.encode_float(&silence[..frame_samples * self.channels as usize], &mut output)?;

            packets.push((
                output[..encoded_len].to_vec(),
                frame_samples as u32,
            ));

            remaining = remaining.saturating_sub(self.frame_size);
        }

        Ok(packets)
    }
}
```

### 3. MP4 Renderer (`mp4.rs`)

```rust
use shiguredo_mp4::{
    boxes::{OpusBox, DopsBox, AudioSampleEntryFields, UnknownBox},
    mux::{Mp4FileMuxer, Sample, SampleEntry, TrackKind},
};

pub struct Mp4Renderer {
    silence_encoder: Option<SilenceEncoder>,
}

impl Mp4Renderer {
    pub fn new() -> Self {
        Self { silence_encoder: None }
    }

    /// Build OpusBox for sample entry
    fn create_opus_box(&self, info: &OpusStreamInfo) -> OpusBox {
        OpusBox {
            audio: AudioSampleEntryFields {
                channel_count: info.channels,
                sample_size: 16,  // Opus uses 16-bit internal
                sample_rate: info.sample_rate,
            },
            dops_box: DopsBox {
                output_channel_count: info.channels as u8,
                pre_skip: 312,  // Standard Opus encoder delay
                input_sample_rate: info.sample_rate,
                output_gain: 0,
            },
            unknown_boxes: Vec::new(),
        }
    }
}

#[async_trait]
impl AudioRenderer for Mp4Renderer {
    async fn render(
        &mut self,
        session_path: &Path,
        player_name: &str,
        output_path: &Path,
    ) -> Result<(), anyhow::Error> {
        // Create packet stream
        let stream = OpusPacketStream::new(session_path, player_name)?;
        let info = stream.info()
            .ok_or_else(|| anyhow::anyhow!("No audio data for player: {}", player_name))?
            .clone();

        // Initialize silence encoder
        self.silence_encoder = Some(SilenceEncoder::new(info.sample_rate, info.channels)?);

        // Create muxer and output file
        let mut muxer = Mp4FileMuxer::new();
        let mut file = std::fs::File::create(output_path)?;

        // Write initial boxes (ftyp, free space for moov)
        use std::io::Write;
        file.write_all(&muxer.initial_boxes_bytes())?;

        // Track file position for mdat
        let mut data_offset = std::io::Seek::stream_position(&mut file)?;

        // Prepare sample entry (only needed for first sample)
        let opus_box = self.create_opus_box(&info);
        let mut first_sample = true;

        // Process all packets
        for chunk in stream {
            match chunk? {
                OpusChunk::Packet { data, duration_samples } => {
                    // Write raw Opus data
                    file.write_all(&data)?;

                    // Register with muxer
                    muxer.append_sample(&Sample {
                        track_kind: TrackKind::Audio,
                        sample_entry: if first_sample {
                            first_sample = false;
                            Some(SampleEntry::Opus(opus_box.clone()))
                        } else {
                            None
                        },
                        keyframe: false,  // Audio samples are not keyframes
                        timescale: std::num::NonZeroU32::new(info.sample_rate).unwrap(),
                        duration: duration_samples,
                        data_offset,
                        data_size: data.len(),
                    })?;

                    data_offset += data.len() as u64;
                }

                OpusChunk::Silence { duration_samples } => {
                    // Encode and write silence packets
                    let silence_packets = self.silence_encoder
                        .as_mut()
                        .unwrap()
                        .encode_silence(duration_samples)?;

                    for (packet_data, packet_duration) in silence_packets {
                        file.write_all(&packet_data)?;

                        muxer.append_sample(&Sample {
                            track_kind: TrackKind::Audio,
                            sample_entry: if first_sample {
                                first_sample = false;
                                Some(SampleEntry::Opus(opus_box.clone()))
                            } else {
                                None
                            },
                            keyframe: false,
                            timescale: std::num::NonZeroU32::new(info.sample_rate).unwrap(),
                            duration: packet_duration,
                            data_offset,
                            data_size: packet_data.len(),
                        })?;

                        data_offset += packet_data.len() as u64;
                    }
                }
            }
        }

        // Finalize muxer
        muxer.finalize()?;
        let finalized = muxer.finalized_boxes()
            .ok_or_else(|| anyhow::anyhow!("Muxer not finalized"))?;

        // Write moov box
        file.write_all(&finalized.moov_bytes)?;

        // Write timecode track
        let timecode_bytes = create_timecode_track(&info)?;
        file.write_all(&timecode_bytes)?;

        // Write user data box
        let udta_bytes = create_user_data_box(&info)?;
        file.write_all(&udta_bytes)?;

        log::info!(
            "MP4 render complete for {}: {} bytes written",
            player_name,
            data_offset
        );

        Ok(())
    }

    fn file_extension(&self) -> &str {
        "m4a"
    }
}
```

### 4. Timecode Track (`timecode.rs`)

Per [Apple TN2174](https://developer.apple.com/library/archive/technotes/tn2174/_index.html):

```rust
/// Create timecode track boxes for NLE compatibility
pub fn create_timecode_track(info: &OpusStreamInfo) -> Result<Vec<u8>, anyhow::Error> {
    let mut bytes = Vec::new();

    // Calculate initial timecode from session start timestamp
    let start_ts = info.session_info.start_timestamp;
    let datetime = chrono::DateTime::from_timestamp_millis(start_ts as i64)
        .unwrap_or_else(chrono::Utc::now);

    // Frames since midnight (for audio: sample groups)
    let seconds_since_midnight = datetime.num_seconds_from_midnight();
    let frames_since_midnight = (seconds_since_midnight as u64 * info.sample_rate as u64) / 960; // 20ms frames

    // Build trak box hierarchy
    // trak
    //   tkhd (track header)
    //   mdia
    //     mdhd (media header)
    //     hdlr (handler: tmcd)
    //     minf
    //       nmhd (null media header)
    //       dinf
    //       stbl (sample table)

    // ... box construction using big-endian byte writing ...

    // TimecodeSampleEntry structure:
    // - flags: 0x00000002 (24-hour wrap)
    // - timescale: 48000
    // - frameDuration: 960 (20ms @ 48kHz)
    // - numFrames: 50 (pseudo-framerate)

    Ok(bytes)
}

/// Create user data box with session metadata
pub fn create_user_data_box(info: &OpusStreamInfo) -> Result<Vec<u8>, anyhow::Error> {
    let mut bytes = Vec::new();

    // udta box containing:
    // - Session ID
    // - Start timestamp (Unix ms)
    // - Player name
    // - Duration

    // Build as nested atoms with string/integer payloads

    Ok(bytes)
}
```

**Box Structure for tmcd Track:**
```
trak (track container)
├── tkhd (track header)
│   ├── version/flags
│   ├── track_id
│   ├── duration
│   └── dimensions (0x0 for timecode)
├── mdia (media container)
│   ├── mdhd (media header)
│   │   ├── timescale: 48000
│   │   └── duration
│   ├── hdlr (handler reference)
│   │   ├── handler_type: "tmcd"
│   │   └── name: "TimeCodeHandler"
│   └── minf (media information)
│       ├── nmhd (null media header) ← NOT gmhd like QuickTime
│       ├── dinf (data information)
│       │   └── dref (data reference)
│       └── stbl (sample table)
│           ├── stsd (sample descriptions)
│           │   └── tmcd (TimecodeSampleEntry)
│           │       ├── flags: 0x02
│           │       ├── timescale: 48000
│           │       ├── frameDuration: 960
│           │       └── numFrames: 50
│           ├── stts (time-to-sample)
│           ├── stsc (sample-to-chunk)
│           ├── stsz (sample sizes)
│           └── stco (chunk offsets)
```

### 5. AudioFormat Update (`mod.rs`)

```rust
mod bwav;
mod mp4;
mod opus_stream;
mod pcm_stream;
mod timecode;

pub use bwav::BwavRenderer;
pub use mp4::Mp4Renderer;
pub use opus_stream::{OpusChunk, OpusPacketStream, OpusStreamInfo};
pub use pcm_stream::{PcmChunk, PcmStream, PcmStreamInfo};

/// Audio output format selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../src/js/bindings/")]
pub enum AudioFormat {
    /// Broadcast WAV with BEXT metadata (uncompressed PCM)
    Bwav,
    /// MP4/M4A with Opus audio (compressed, lossless passthrough)
    Mp4Opus,
}

impl AudioFormat {
    /// Returns the file extension (without dot)
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Bwav => "wav",
            AudioFormat::Mp4Opus => "m4a",
        }
    }

    /// Render audio from session to output path
    pub async fn render(
        &self,
        session_path: &Path,
        player_name: &str,
        output_path: &Path,
    ) -> Result<(), anyhow::Error> {
        match self {
            AudioFormat::Bwav => {
                BwavRenderer::new().render(session_path, player_name, output_path).await
            }
            AudioFormat::Mp4Opus => {
                Mp4Renderer::new().render(session_path, player_name, output_path).await
            }
        }
    }
}
```

### 6. Cargo.toml Addition

```toml
[dependencies]
# ... existing dependencies ...

# MP4 muxing with Opus support
shiguredo-mp4 = "0.6"
```

---

## Timecode Calculation

### From Session Start to Time-of-Day

```rust
fn calculate_timecode(session_start_ms: u64, sample_rate: u32) -> TimecodeInfo {
    // Convert Unix timestamp to local time
    let datetime = chrono::DateTime::from_timestamp_millis(session_start_ms as i64)
        .map(|dt| dt.with_timezone(&chrono::Local))
        .unwrap_or_else(chrono::Local::now);

    // Get midnight of the same day
    let midnight = datetime.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let midnight_ts = chrono::Local.from_local_datetime(&midnight).unwrap();

    // Milliseconds since midnight
    let ms_since_midnight = (datetime.timestamp_millis() - midnight_ts.timestamp_millis()) as u64;

    // Convert to sample-based timecode
    // For 48kHz with 20ms frames: 50 "frames" per second
    let total_samples = (ms_since_midnight * sample_rate as u64) / 1000;
    let frame_duration = (sample_rate as u64 * 20) / 1000; // 960 samples
    let frame_count = total_samples / frame_duration;

    TimecodeInfo {
        hours: (frame_count / (50 * 60 * 60)) as u8,
        minutes: ((frame_count / (50 * 60)) % 60) as u8,
        seconds: ((frame_count / 50) % 60) as u8,
        frames: (frame_count % 50) as u8,
        sample_offset: total_samples,
    }
}
```

---

## Testing Plan

### Unit Tests
1. `OpusPacketStream` correctly yields raw packets
2. `SilenceEncoder` produces valid Opus packets
3. Timecode calculation matches expected values
4. Box construction produces valid MP4 structures

### Integration Tests
1. Render existing WAL recording to M4A
2. Verify playback in VLC, QuickTime
3. Verify timecode in Final Cut Pro, DaVinci Resolve
4. Compare file size vs BWAV equivalent
5. Verify audio quality (should be bit-exact with source)

### Validation Commands
```bash
# Check MP4 structure
ffprobe -show_format -show_streams output.m4a

# Verify Opus codec
ffprobe -show_entries stream=codec_name output.m4a

# Check timecode track
ffprobe -show_entries stream=codec_type,codec_tag_string output.m4a

# Extract and compare audio
ffmpeg -i output.m4a -f opus extracted.opus
```

---

## Implementation Order

1. **Add dependency** - `shiguredo-mp4` to Cargo.toml
2. **Create OpusPacketStream** - Raw packet iterator in `opus_stream.rs`
3. **Create SilenceEncoder** - Silence packet generation
4. **Create Mp4Renderer** - Basic muxing without timecode
5. **Add timecode track** - tmcd/nmhd box construction
6. **Add udta metadata** - Session info in user data
7. **Update AudioFormat** - Add Mp4Opus variant
8. **Test** - Verify output in various players/NLEs

---

## References

- [shiguredo/mp4-rust](https://github.com/shiguredo/mp4-rust) - MP4 library with Opus support
- [Apple TN2174](https://developer.apple.com/library/archive/technotes/tn2174/_index.html) - Timecode in MP4 for Final Cut Pro
- [Opus in ISOBMFF](https://opus-codec.org/docs/opus_in_isobmff.html) - Opus encapsulation specification
- [RFC 7845](https://datatracker.ietf.org/doc/html/rfc7845) - Ogg Opus (pre_skip reference)
