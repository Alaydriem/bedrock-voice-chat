use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

use common::Game;

// Long enough for the client to receive the ticket and open the socket, short
// enough that a ticket captured from a log has already expired.
const TICKET_TTL: Duration = Duration::from_secs(60);
const TICKET_CAPACITY: u64 = 4096;

/// The authenticated identity a redeemed ticket confers on a socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketIdentity {
    pub gamertag: String,
    pub game: Game,
}

/// Single-use exchange tokens for WebSocket upgrades.
///
/// The browser WebSocket API cannot set request headers and cannot present a
/// client certificate, so mTLS -- the identity every other API route relies on
/// -- is unavailable at upgrade time. The client proves itself once over mTLS
/// to obtain a ticket, then spends it on the socket. Single-use and
/// short-lived, so it is an exchange token rather than a credential worth
/// stealing.
///
/// Not specific to any one feed: every WebSocket route uses this exchange.
#[derive(Clone)]
pub struct WebsocketTicketCache {
    // ticket -> identity it was issued to
    cache: Arc<Cache<String, TicketIdentity>>,
    // identity -> its one outstanding ticket
    outstanding: Arc<Cache<String, String>>,
}

impl WebsocketTicketCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(
                Cache::builder()
                    .time_to_live(TICKET_TTL)
                    .max_capacity(TICKET_CAPACITY)
                    .build(),
            ),
            outstanding: Arc::new(
                Cache::builder()
                    .time_to_live(TICKET_TTL)
                    .max_capacity(TICKET_CAPACITY)
                    .build(),
            ),
        }
    }

    /// Issues a ticket, superseding any the identity already holds.
    ///
    /// The store is shared and bounded, so an identity able to accumulate
    /// tickets could evict everyone else's and deny them their upgrade. Holding
    /// each identity to one outstanding ticket makes that structurally
    /// impossible, which a request-rate limit could only approximate -- and it
    /// would have to key on client IP, punishing shared addresses while doing
    /// nothing about an authenticated caller who changes theirs.
    ///
    /// It also cleans up after an abandoned connection attempt rather than
    /// leaving a dead ticket to occupy a slot until it expires.
    pub async fn issue(&self, identity: TicketIdentity) -> String {
        let key = Self::identity_key(&identity);

        if let Some(previous) = self.outstanding.get(&key).await {
            self.cache.remove(&previous).await;
        }

        let ticket = nanoid::nanoid!(32);
        self.cache.insert(ticket.clone(), identity).await;
        self.outstanding.insert(key, ticket.clone()).await;

        ticket
    }

    /// Consumes the ticket and returns the identity bound to it at issue time.
    pub async fn redeem(&self, ticket: &str) -> Option<TicketIdentity> {
        let identity = self.cache.get(ticket).await?;
        self.cache.remove(ticket).await;

        // Clear the reverse entry only while it still names this ticket: a
        // newer issue may already have replaced it, and dropping that would
        // let the identity hold two outstanding tickets again. A concurrent
        // issue can still leave a stale entry here, which costs nothing --
        // the next issue overwrites it and the TTL reclaims it regardless.
        let key = Self::identity_key(&identity);
        if self.outstanding.get(&key).await.as_deref() == Some(ticket) {
            self.outstanding.remove(&key).await;
        }

        Some(identity)
    }

    // Matches the `game:gamertag` identity convention used for channel
    // membership and certificate common names, so one gamertag on two games is
    // two identities.
    fn identity_key(identity: &TicketIdentity) -> String {
        format!("{}:{}", identity.game.as_str(), identity.gamertag)
    }
}

impl Default for WebsocketTicketCache {
    fn default() -> Self {
        Self::new()
    }
}
