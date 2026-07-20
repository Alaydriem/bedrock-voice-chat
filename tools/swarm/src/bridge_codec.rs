use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Wire codec for the `bvc_client_e2e` stdio bridge: a `u32` big-endian length
/// prefix followed by a serde_json body. This mirrors `bvc_client_lib`'s
/// `testkit::bridge::Frame` without linking the client crate; messages are moved
/// as `serde_json::Value` so the swarm never drifts from the client's enum types.
pub struct BridgeCodec;

impl BridgeCodec {
    const MAX_FRAME_LEN: u32 = 64 * 1024 * 1024;

    pub async fn write<W: AsyncWriteExt + Unpin>(
        writer: &mut W,
        value: &serde_json::Value,
    ) -> std::io::Result<()> {
        let body = serde_json::to_vec(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let len = u32::try_from(body.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "frame body too large")
        })?;
        writer.write_all(&len.to_be_bytes()).await?;
        writer.write_all(&body).await?;
        writer.flush().await
    }

    pub async fn read<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<serde_json::Value> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf);
        if len > Self::MAX_FRAME_LEN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("frame length {} exceeds maximum {}", len, Self::MAX_FRAME_LEN),
            ));
        }
        let mut body = vec![0u8; len as usize];
        reader.read_exact(&mut body).await?;
        serde_json::from_slice(&body)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
