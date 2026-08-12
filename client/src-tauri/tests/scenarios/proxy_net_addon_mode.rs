use std::time::Duration;

use common::structs::bedrock::AddonMode;

use crate::harness::protocol_matrix::ProtocolMatrix;
use crate::harness::proxy_world::ProxyWorld;

// How long to watch the serverbound stream for an in-band ride. Long enough to
// cover the presence and eject paths, which fire on demand rather than on the
// relay's 60 s announce cycle.
const CAPTURE_WINDOW: Duration = Duration::from_millis(1500);

/// A net-mode session must not ride state into the world as chat. The addon on
/// such a world cancels `!bvcp` and `!bvca` before any peer can observe them, so
/// the ride is pure waste — and on a world whose addon is momentarily absent it
/// would surface in public chat under the player's own name.
#[tokio::test(flavor = "multi_thread")]
async fn net_mode_sends_no_in_band_rides_upstream() {
    let v = ProtocolMatrix::last_two()
        .into_iter()
        .next()
        .expect("at least one protocol version");
    let mut w = ProxyWorld::boot_with_mode(v, &["Alice"], AddonMode::Net).await;

    for _ in 0..5 {
        w.upstream.drive_position("Alice", 0.0, 64.0, 0.0).await;
        tokio::time::sleep(Duration::from_millis(120)).await;
    }

    // Nothing at all, not merely no `!bvc` prefixes: chat egress is suppressed
    // in this mode too, so a relay-only session has no reason to author any
    // serverbound chat whatsoever.
    let text = w.upstream.drain_serverbound_chat("Alice", CAPTURE_WINDOW).await;
    assert!(
        text.is_empty(),
        "[{v}] a relay-only session must send nothing serverbound as chat, got: {text:?}"
    );

    w.shutdown();
}

// There is deliberately no positive control here. On its own the test above
// would pass just as well against a proxy that sends nothing at all, so the
// suppression needs a counterpart proving no-net still rides — but driving a
// ride takes the full jukebox playback rig, and `proxy_jukebox`, `relay_jukebox`
// and `position_feed` already exercise every one of these paths under the
// default `NoNet`. Those three are the positive control; run them with this.
