use bvc_server_lib::stream::quic::connection::RoutedPacket;
use bvc_server_lib::stream::quic::stream_manager::SendBatcher;
use bytes::Bytes;
use tokio::sync::mpsc;

fn routed(byte: u8) -> RoutedPacket {
    RoutedPacket::Serialized(Bytes::from(vec![byte]))
}

#[tokio::test]
async fn zero_wait_flushes_without_delay_but_still_drains_backlog() {
    let (tx, mut rx) = mpsc::channel(16);
    tx.send(routed(1)).await.unwrap();
    tx.send(routed(2)).await.unwrap();

    let batcher = SendBatcher::new(0);
    let mut out = Vec::new();
    batcher.collect(&mut rx, &mut out).await.unwrap();
    // Both were already queued, so even a zero wait drains them into one flush.
    assert_eq!(out.len(), 2);
}

#[tokio::test(start_paused = true)]
async fn wait_window_accumulates_late_arrivals() {
    let (tx, mut rx) = mpsc::channel(16);
    tx.send(routed(1)).await.unwrap();

    let sender = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_micros(500)).await;
        tx.send(routed(2)).await.unwrap();
        // Keep tx alive until after the batch window closes.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    });

    let batcher = SendBatcher::new(2_000);
    let mut out = Vec::new();
    batcher.collect(&mut rx, &mut out).await.unwrap();
    assert_eq!(out.len(), 2);
    sender.await.unwrap();
}

#[tokio::test]
async fn closed_channel_ends_collection() {
    let (tx, mut rx) = mpsc::channel::<RoutedPacket>(4);
    drop(tx);
    let batcher = SendBatcher::new(0);
    let mut out = Vec::new();
    assert!(batcher.collect(&mut rx, &mut out).await.is_none());
}

#[tokio::test]
async fn one_flush_is_bounded() {
    let (tx, mut rx) = mpsc::channel(128);
    for i in 0..100u8 {
        tx.send(routed(i)).await.unwrap();
    }
    let batcher = SendBatcher::new(0);
    let mut out = Vec::new();
    batcher.collect(&mut rx, &mut out).await.unwrap();
    assert_eq!(out.len(), 32);
    // The remainder is still queued for the next flush, not dropped.
    batcher.collect(&mut rx, &mut out).await.unwrap();
    assert_eq!(out.len(), 32);
}
