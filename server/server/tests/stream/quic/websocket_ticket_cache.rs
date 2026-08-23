use bvc_server_lib::stream::quic::{CacheManager, TicketIdentity};
use common::Game;

fn identity(name: &str) -> TicketIdentity {
    TicketIdentity {
        gamertag: name.to_string(),
        game: Game::Minecraft,
    }
}

// The ticket is an exchange token, not a credential: redeeming must consume it,
// so a copy captured anywhere cannot open a second socket.
#[tokio::test]
async fn a_ticket_redeems_exactly_once() {
    let cache = CacheManager::new();
    let tickets = cache.websocket_tickets();
    let ticket = tickets.issue(identity("Alice")).await;

    let first = tickets.redeem(&ticket).await;
    let second = tickets.redeem(&ticket).await;

    assert_eq!(first.map(|i| i.gamertag), Some("Alice".to_string()));
    assert!(second.is_none(), "a redeemed ticket must not redeem again");
}

// The socket's identity comes from the ticket and nothing the client sends, so
// redemption must return the identity bound at issue time.
#[tokio::test]
async fn redeeming_returns_the_identity_bound_at_issue() {
    let cache = CacheManager::new();
    let tickets = cache.websocket_tickets();
    let alice = tickets.issue(identity("Alice")).await;
    let bob = tickets.issue(identity("Bob")).await;

    assert_eq!(tickets.redeem(&bob).await.unwrap().gamertag, "Bob");
    assert_eq!(tickets.redeem(&alice).await.unwrap().gamertag, "Alice");
}

#[tokio::test]
async fn an_unknown_ticket_is_rejected() {
    let cache = CacheManager::new();

    assert!(
        cache
            .websocket_tickets()
            .redeem("not-a-real-ticket")
            .await
            .is_none()
    );
}

// A repeated ticket would let one player's socket bind to another's identity.
#[tokio::test]
async fn issued_tickets_are_unique() {
    let cache = CacheManager::new();
    let tickets = cache.websocket_tickets();

    let a = tickets.issue(identity("Alice")).await;
    let b = tickets.issue(identity("Alice")).await;

    assert_ne!(a, b);
}

// The store is shared and bounded, so an identity able to accumulate tickets can
// evict everyone else's. Replacing on issue holds one identity to one
// outstanding ticket, which makes that impossible rather than merely throttled.
#[tokio::test]
async fn issuing_a_second_ticket_invalidates_the_first() {
    let cache = CacheManager::new();
    let tickets = cache.websocket_tickets();

    let first = tickets.issue(identity("Alice")).await;
    let second = tickets.issue(identity("Alice")).await;

    assert!(
        tickets.redeem(&first).await.is_none(),
        "the superseded ticket must no longer redeem"
    );
    assert_eq!(
        tickets.redeem(&second).await.map(|i| i.gamertag),
        Some("Alice".to_string()),
        "the newest ticket must still redeem"
    );
}

// Replacement is scoped to one identity: a reconnecting player must not cancel
// anyone else's pending upgrade.
#[tokio::test]
async fn replacement_does_not_disturb_another_identity() {
    let cache = CacheManager::new();
    let tickets = cache.websocket_tickets();

    let bob = tickets.issue(identity("Bob")).await;
    tickets.issue(identity("Alice")).await;
    tickets.issue(identity("Alice")).await;

    assert_eq!(
        tickets.redeem(&bob).await.map(|i| i.gamertag),
        Some("Bob".to_string())
    );
}

// Every clone of the manager shares one ticket store. Rocket hands each route a
// clone, so a ticket issued while serving the mint request has to be redeemable
// while serving the upgrade.
#[tokio::test]
async fn tickets_are_shared_across_manager_clones() {
    let issuer = CacheManager::new();
    let redeemer = issuer.clone();

    let ticket = issuer.websocket_tickets().issue(identity("Alice")).await;

    assert_eq!(
        redeemer
            .websocket_tickets()
            .redeem(&ticket)
            .await
            .map(|i| i.gamertag),
        Some("Alice".to_string())
    );
}

// The sequential sibling above cannot observe the failure this guards. Two upgrades
// arriving together both read the ticket before either consumes it, and both are handed
// the identity -- so redemption has to be one atomic operation, and only a race can
// demonstrate that it is. The barrier releases every task at once rather than trusting the
// scheduler to interleave them, and the trials make one unlucky ordering insufficient to
// pass.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_redemptions_yield_exactly_one_identity() {
    const RACERS: usize = 32;
    const TRIALS: usize = 25;

    for trial in 0..TRIALS {
        let cache = CacheManager::new().websocket_tickets().clone();
        let ticket = cache.issue(identity("Alice")).await;
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(RACERS));

        let mut racers = tokio::task::JoinSet::new();
        for _ in 0..RACERS {
            let cache = cache.clone();
            let ticket = ticket.clone();
            let barrier = barrier.clone();
            racers.spawn(async move {
                barrier.wait().await;
                cache.redeem(&ticket).await.is_some()
            });
        }

        let mut redeemed = 0;
        while let Some(result) = racers.join_next().await {
            if result.expect("racer panicked") {
                redeemed += 1;
            }
        }

        assert_eq!(
            redeemed, 1,
            "trial {trial}: one ticket must confer identity exactly once, not {redeemed} times"
        );
    }
}

// The one-outstanding-per-identity rule is the design's stated DoS bound. Concurrent mints
// that each read the same predecessor all remove that one and leave their own behind,
// unreachable by any later supersede and live until its TTL -- which is the accumulation
// the bound exists to prevent.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_issues_leave_exactly_one_live_ticket() {
    const RACERS: usize = 32;
    const TRIALS: usize = 25;

    for trial in 0..TRIALS {
        let cache = CacheManager::new().websocket_tickets().clone();
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(RACERS));

        let mut racers = tokio::task::JoinSet::new();
        for _ in 0..RACERS {
            let cache = cache.clone();
            let barrier = barrier.clone();
            racers.spawn(async move {
                barrier.wait().await;
                cache.issue(identity("Alice")).await
            });
        }

        let mut tickets = Vec::with_capacity(RACERS);
        while let Some(result) = racers.join_next().await {
            tickets.push(result.expect("racer panicked"));
        }

        let mut live = 0;
        for ticket in &tickets {
            if cache.redeem(ticket).await.is_some() {
                live += 1;
            }
        }

        assert_eq!(
            live, 1,
            "trial {trial}: one identity must hold one outstanding ticket, not {live}"
        );
    }
}
