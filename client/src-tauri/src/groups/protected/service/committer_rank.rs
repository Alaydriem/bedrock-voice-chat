use std::time::Duration;

/// Who commits a shared duty, and after how long.
///
/// Every member computes the same order from state they all hold, so the first member in line
/// acts at once and each later one waits [`Self::FALLBACK`] longer, acting only if nobody before
/// it did. No coordination message is needed.
pub struct CommitterRank;

impl CommitterRank {
    pub const FALLBACK: Duration = Duration::from_secs(3);

    /// Delay before `own` removes a gone member: lowest remaining leaf first.
    pub fn removal_delay(leaves: &[u32], gone: u32, own: u32) -> Option<Duration> {
        let mut order: Vec<u32> = leaves.iter().copied().filter(|l| *l != gone).collect();
        order.sort_unstable();
        let rank = order.iter().position(|l| *l == own)?;
        Some(Self::FALLBACK * rank as u32)
    }

    /// Delay past the periodic deadline before `own` commits the ratchet. The first in line moves
    /// with the epoch, so the duty rotates through members as epochs pass.
    pub fn periodic_delay(leaves: &[u32], epoch: u64, own: u32) -> Option<Duration> {
        let mut order = leaves.to_vec();
        order.sort_unstable();
        if order.is_empty() {
            return None;
        }
        let designated = (epoch % order.len() as u64) as usize;
        let position = order.iter().position(|l| *l == own)?;
        let rank = (position + order.len() - designated) % order.len();
        Some(Self::FALLBACK * rank as u32)
    }
}
