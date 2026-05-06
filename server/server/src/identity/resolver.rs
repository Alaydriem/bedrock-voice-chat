use anyhow::anyhow;

use super::store::IdentityStore;
use super::IdentitySlot;

pub struct IdentityResolver;

impl IdentityResolver {
    pub fn active(explicit: Option<&str>) -> Result<IdentitySlot, anyhow::Error> {
        if let Some(value) = explicit {
            return IdentitySlot::parse(value);
        }
        if let Ok(value) = std::env::var("BVC_IDENTITY") {
            return IdentitySlot::parse(&value);
        }

        let mut summaries = IdentityStore::list()?;
        match summaries.len() {
            0 => Err(anyhow!(
                "no identity found. Run `bvc login` first, or pass --identity <gamertag>:<game>"
            )),
            1 => {
                let summary = summaries.remove(0);
                Ok(summary.slot)
            }
            _ => {
                let names: Vec<String> = summaries
                    .iter()
                    .map(|s| s.slot.key())
                    .collect();
                Err(anyhow!(
                    "ambiguous identity ({} stored: {}). Set BVC_IDENTITY or pass --identity",
                    summaries.len(),
                    names.join(", ")
                ))
            }
        }
    }
}
