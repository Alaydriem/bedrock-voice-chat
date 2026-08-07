use bvc_client_lib::audio::JobSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const GRACE: Duration = Duration::from_millis(300);

#[tokio::test]
async fn an_empty_set_settles_without_waiting() {
    let mut jobs = JobSet::empty();

    let started = Instant::now();
    let finished_on_its_own = jobs.settle(GRACE).await;

    assert!(finished_on_its_own);
    assert!(
        started.elapsed() < GRACE,
        "an empty settle took {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_set_settles_when_its_jobs_end_not_when_the_window_does() {
    let done = Arc::new(AtomicU32::new(0));

    let mut jobs = JobSet::from(
        (0..3)
            .map(|_| {
                let done = done.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    done.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect::<Vec<_>>(),
    );

    let started = Instant::now();
    let finished_on_its_own = jobs.settle(GRACE).await;

    assert!(finished_on_its_own);
    assert_eq!(done.load(Ordering::SeqCst), 3, "not every job was waited for");
    assert!(
        started.elapsed() < GRACE,
        "settle took the whole window ({:?}) for work that ended early",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_job_that_will_not_finish_is_aborted_at_the_window() {
    let reached_end = Arc::new(AtomicU32::new(0));
    let marker = reached_end.clone();

    let mut jobs = JobSet::from(vec![tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        marker.fetch_add(1, Ordering::SeqCst);
    })]);

    let started = Instant::now();
    let finished_on_its_own = jobs.settle(GRACE).await;
    let elapsed = started.elapsed();

    assert!(!finished_on_its_own);
    assert!(elapsed >= GRACE, "settle gave up early at {elapsed:?}");
    assert!(
        elapsed < GRACE * 4,
        "settle waited far past its window ({elapsed:?})"
    );
    assert_eq!(reached_end.load(Ordering::SeqCst), 0, "the job was not killed");
}

#[tokio::test]
async fn a_settled_set_is_empty() {
    let mut jobs = JobSet::from(vec![tokio::spawn(async {})]);
    assert!(!jobs.is_empty());

    jobs.settle(GRACE).await;

    assert!(jobs.is_empty());
}

#[tokio::test]
async fn settling_twice_is_harmless() {
    let mut jobs = JobSet::from(vec![tokio::spawn(async {})]);
    jobs.settle(GRACE).await;

    let started = Instant::now();
    assert!(jobs.settle(GRACE).await);
    assert!(started.elapsed() < GRACE);
}
