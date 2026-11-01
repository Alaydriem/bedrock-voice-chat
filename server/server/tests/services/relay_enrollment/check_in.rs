//! When a check-in happens, apart from what it says. The registry answers a check-in inside
//! its one-minute window without probing, and probes an address the moment it hears it.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use bvc_server_lib::services::RelayCheckIn;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

fn counting(count: &Arc<AtomicU32>) -> impl FnMut() -> std::future::Ready<()> + Send + 'static {
    let count = count.clone();
    move || {
        count.fetch_add(1, Ordering::SeqCst);
        std::future::ready(())
    }
}

#[tokio::test(start_paused = true)]
async fn the_follow_up_checks_in_once_outside_the_registry_window() {
    let count = Arc::new(AtomicU32::new(0));
    let cancel = CancellationToken::new();
    let follow_up = RelayCheckIn::REGISTRY_WINDOW + Duration::from_secs(5);
    let handle = RelayCheckIn::spawn_schedule(cancel.clone(), Some(follow_up), None, counting(&count));

    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1, "the first check-in is at once");

    tokio::time::sleep(Duration::from_secs(60)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1, "nothing inside the registry's window");

    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(count.load(Ordering::SeqCst), 2, "the follow-up lands outside the window");

    tokio::time::sleep(Duration::from_secs(3000)).await;
    assert_eq!(count.load(Ordering::SeqCst), 2, "the follow-up happens once");

    tokio::time::sleep(Duration::from_secs(700)).await;
    assert_eq!(count.load(Ordering::SeqCst), 3, "then the hourly check-in");

    cancel.cancel();
    handle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn no_check_in_happens_before_the_server_is_listening() {
    let count = Arc::new(AtomicU32::new(0));
    let cancel = CancellationToken::new();
    let (listening_tx, listening_rx) = watch::channel(false);
    let handle = RelayCheckIn::spawn_schedule(
        cancel.clone(),
        Some(Duration::from_secs(65)),
        Some(listening_rx),
        counting(&count),
    );

    tokio::time::sleep(Duration::from_secs(120)).await;
    assert_eq!(count.load(Ordering::SeqCst), 0, "the registry would probe a closed port");

    listening_tx.send_replace(true);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1);

    cancel.cancel();
    handle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn cancelling_before_the_server_listens_ends_the_schedule() {
    let count = Arc::new(AtomicU32::new(0));
    let cancel = CancellationToken::new();
    let (_listening_tx, listening_rx) = watch::channel(false);
    let handle = RelayCheckIn::spawn_schedule(cancel.clone(), None, Some(listening_rx), counting(&count));

    cancel.cancel();
    handle.await.unwrap();

    assert_eq!(count.load(Ordering::SeqCst), 0);
}
