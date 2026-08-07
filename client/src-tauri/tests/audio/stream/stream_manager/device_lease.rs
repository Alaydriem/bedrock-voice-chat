use bvc_client_lib::audio::DeviceLease;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

const RELEASE_COST: Duration = Duration::from_millis(150);

// Stands in for a cpal stream, which cannot be built without a real device.
struct SlowHandle {
    released: Arc<AtomicBool>,
    cost: Duration,
}

impl SlowHandle {
    fn new(cost: Duration) -> (Self, Arc<AtomicBool>) {
        let released = Arc::new(AtomicBool::new(false));
        (
            Self {
                released: released.clone(),
                cost,
            },
            released,
        )
    }
}

impl Drop for SlowHandle {
    fn drop(&mut self) {
        std::thread::sleep(self.cost);
        self.released.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn release_does_not_return_until_the_device_is_back() {
    let (handle, released) = SlowHandle::new(RELEASE_COST);
    let mut lease = DeviceLease::empty();
    lease.hold(Some(handle)).await;

    lease.release().await;

    assert!(
        released.load(Ordering::SeqCst),
        "release returned while the handle was still alive; the next open would race it"
    );
    assert!(!lease.is_held());
}

#[tokio::test]
async fn a_lease_holding_nothing_releases_without_waiting() {
    let mut lease = DeviceLease::<SlowHandle>::empty();

    let started = Instant::now();
    lease.release().await;

    assert!(
        started.elapsed() < RELEASE_COST,
        "an empty release took {:?}, so something is still being waited on",
        started.elapsed()
    );
    assert!(!lease.is_held());
}

#[tokio::test]
async fn holding_a_new_handle_releases_the_one_it_displaces() {
    let (first, first_released) = SlowHandle::new(RELEASE_COST);
    let (second, second_released) = SlowHandle::new(RELEASE_COST);

    let mut lease = DeviceLease::empty();
    lease.hold(Some(first)).await;
    lease.hold(Some(second)).await;

    assert!(
        first_released.load(Ordering::SeqCst),
        "the displaced handle still holds its device"
    );
    assert!(!second_released.load(Ordering::SeqCst));
    assert!(lease.is_held());
}

#[tokio::test]
async fn releasing_twice_releases_once() {
    let (handle, released) = SlowHandle::new(RELEASE_COST);
    let mut lease = DeviceLease::empty();
    lease.hold(Some(handle)).await;

    lease.release().await;
    let started = Instant::now();
    lease.release().await;

    assert!(released.load(Ordering::SeqCst));
    assert!(
        started.elapsed() < RELEASE_COST,
        "the second release waited on something"
    );
}

#[tokio::test]
async fn a_slow_release_leaves_the_runtime_free() {
    let (handle, _) = SlowHandle::new(RELEASE_COST);
    let mut lease = DeviceLease::empty();
    lease.hold(Some(handle)).await;

    let ticks = Arc::new(AtomicU32::new(0));
    let counter = ticks.clone();
    let ticker = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(10)).await;
            counter.fetch_add(1, Ordering::SeqCst);
        }
    });

    lease.release().await;
    ticker.abort();

    assert!(
        ticks.load(Ordering::SeqCst) > 0,
        "nothing else ran during the release, so the drop was blocking the runtime"
    );
}
