use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

mod in_msg;
mod out_msg;

pub use in_msg::InMsg;
pub use out_msg::OutMsg;

// Upper bound on a single frame so a corrupt length prefix cannot trigger a
// multi-gigabyte allocation. PCM chunks are small; 64 MiB is generous.
const MAX_FRAME_LEN: u32 = 64 * 1024 * 1024;

pub struct Frame;

impl Frame {
    pub fn read<R: Read, T: for<'de> Deserialize<'de>>(reader: &mut R) -> std::io::Result<T> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf);
        if len > MAX_FRAME_LEN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("frame length {len} exceeds maximum {MAX_FRAME_LEN}"),
            ));
        }

        let mut body = vec![0u8; len as usize];
        reader.read_exact(&mut body)?;
        serde_json::from_slice(&body)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn write<W: Write, T: Serialize>(writer: &mut W, value: &T) -> std::io::Result<()> {
        let body = serde_json::to_vec(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let len = u32::try_from(body.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "frame body too large")
        })?;
        writer.write_all(&len.to_be_bytes())?;
        writer.write_all(&body)?;
        writer.flush()
    }
}
