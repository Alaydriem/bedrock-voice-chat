use anyhow::anyhow;
use rand::TryRng;
use rand::rngs::SysRng;

/// Nonces and check ids, drawn straight from the operating system's generator.
pub struct OsEntropy;

impl OsEntropy {
    pub fn bytes<const N: usize>() -> anyhow::Result<[u8; N]> {
        let mut out = [0u8; N];
        SysRng
            .try_fill_bytes(&mut out)
            .map_err(|e| anyhow!("the system random generator failed: {e}"))?;
        Ok(out)
    }
}
