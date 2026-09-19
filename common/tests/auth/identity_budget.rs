use common::auth::{AuthError, IdentityBudget};
use std::time::Duration;

// The bound exists so a provider that never answers cannot hold the request past the point
// where its answer could still reach a waiting client.
#[tokio::test(start_paused = true)]
async fn an_exchange_that_outruns_the_budget_is_reported_as_a_network_fault() {
    let result = IdentityBudget::enforce(async {
        tokio::time::sleep(Duration::from_secs(600)).await;
        Ok::<(), AuthError>(())
    })
    .await;

    assert!(
        matches!(result, Err(AuthError::Network(_))),
        "expected a network fault, got {:?}",
        result
    );
}

#[tokio::test(start_paused = true)]
async fn an_exchange_inside_the_budget_returns_its_own_answer() {
    let result = IdentityBudget::enforce(async { Ok::<u8, AuthError>(7) }).await;

    assert!(matches!(result, Ok(7)), "got {:?}", result);
}

// The route picks its HTTP status from the variant, so the variant is the contract.
// Rewriting a refusal as a network fault would tell a player to retry a code the provider
// has already spent.
#[tokio::test(start_paused = true)]
async fn a_refusal_inside_the_budget_is_passed_through_unchanged() {
    let result = IdentityBudget::enforce(async {
        Err::<(), AuthError>(AuthError::CodeRejected("spent".to_string()))
    })
    .await;

    assert!(
        matches!(result, Err(AuthError::CodeRejected(_))),
        "got {:?}",
        result
    );
}
