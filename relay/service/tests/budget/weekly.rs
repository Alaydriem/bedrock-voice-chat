use std::sync::Arc;

use bvc_relay_service::budget::WeeklyBudget;
use bvc_relay_service::db::Db;

// Renewals are the exempt category at the certificate authority, so they must not
// draw on the weekly ceiling. Counting them would make a busy deployment refuse new
// operators while its existing ones renewed.
#[tokio::test]
async fn a_renewal_does_not_draw_on_the_weekly_ceiling() {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let budget = WeeklyBudget::new(conn, 2);

    budget
        .record("name-a", true)
        .await
        .expect("records a renewal");
    budget
        .record("name-b", true)
        .await
        .expect("records a renewal");
    budget
        .record("name-c", true)
        .await
        .expect("records a renewal");

    assert_eq!(budget.remaining().await.expect("reads"), 2);
}

#[tokio::test]
async fn a_first_issuance_draws_on_the_weekly_ceiling() {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let budget = WeeklyBudget::new(conn, 2);

    budget.record("name-a", false).await.expect("records");

    assert_eq!(budget.remaining().await.expect("reads"), 1);
}

// At the ceiling a new issuance is refused rather than attempted. An attempt that
// the authority rejects burns the order and delays the operator further.
#[tokio::test]
async fn a_new_issuance_is_refused_at_the_ceiling() {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let budget = WeeklyBudget::new(conn, 1);
    budget.record("name-a", false).await.expect("records");

    assert!(!budget.admits_new_issuance().await.expect("reads"));
}

// A ceiling already overrun reports nothing remaining rather than underflowing. The
// count is a `u32` and the ceiling can be lowered by configuration below what has
// already been spent.
#[tokio::test]
async fn a_ceiling_lowered_below_what_is_spent_reports_none_remaining() {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let generous = WeeklyBudget::new(conn.clone(), 10);
    for name in ["a", "b", "c"] {
        generous.record(name, false).await.expect("records");
    }

    let tightened = WeeklyBudget::new(conn, 1);

    assert_eq!(tightened.remaining().await.expect("reads"), 0);
}

// Whether a name is renewing is what decides if it draws on the ceiling, so the
// question has to be answerable before the record is written.
#[tokio::test]
async fn a_name_that_has_issued_before_is_recognised_as_renewing() {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let budget = WeeklyBudget::new(conn, 10);

    assert!(!budget.has_issued("name-a").await.expect("reads"));

    budget.record("name-a", false).await.expect("records");

    assert!(budget.has_issued("name-a").await.expect("reads"));
}
