// Roles cached locally are valid for 30 days from the last successful sync.
pub const CACHE_TTL_SECS: i64 = 2_592_000;

#[derive(Debug, Clone, Default)]
pub struct DiscordTraitState {
    pub roles: Vec<String>,
    pub last_sync: Option<i64>,
}

impl DiscordTraitState {
    pub fn new() -> Self {
        Self::default()
    }

    // Roles to send as traits, applying the 30-day cache rule. Empty when never
    // synced or expired so gated features fail-closed.
    pub fn effective_roles(&self, now_secs: i64) -> Vec<String> {
        match self.last_sync {
            Some(t) if now_secs.saturating_sub(t) <= CACHE_TTL_SECS => self.roles.clone(),
            _ => Vec::new(),
        }
    }

    pub fn is_expired(&self, now_secs: i64) -> bool {
        match self.last_sync {
            Some(t) => now_secs.saturating_sub(t) > CACHE_TTL_SECS,
            None => true,
        }
    }
}
