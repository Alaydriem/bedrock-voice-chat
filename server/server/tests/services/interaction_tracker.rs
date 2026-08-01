use bvc_server_lib::services::metrics_service::interaction::InteractionRoute;
use bvc_server_lib::services::metrics_service::interaction::InteractionTracker;

fn h(name: &str) -> u64 {
    InteractionTracker::hash_name(name)
}

// The metric counts distinct participants, not frames. Audio arrives ~50/sec per
// speaker, so a per-frame count would report throughput and call it reach.
#[test]
fn counts_distinct_participants_not_frames() {
    let t = InteractionTracker::new();
    for _ in 0..100 {
        t.record_delivery(InteractionRoute::Proximity, h("alice"), h("bob"));
    }
    assert_eq!(t.counts(InteractionRoute::Proximity).reached, 2);
}

// Each sample must cover only the interval since the previous one, so a pair still
// talking is re-counted in the next window rather than carried forward. Note this
// makes samples non-overlapping, NOT summable: these are distinct-player counts, so
// a pair active all day contributes 2 to each of 96 windows and nothing downstream
// may add them.
#[test]
fn a_pair_from_the_previous_window_does_not_count_in_the_next() {
    let t = InteractionTracker::new();
    t.record_delivery(InteractionRoute::Proximity, h("alice"), h("bob"));
    let first = t.close_window();

    assert_eq!(first[0].1.reached, 2);
    assert_eq!(t.counts(InteractionRoute::Proximity).reached, 0);

    let second = t.close_window();
    assert_eq!(second[0].1.reached, 0);
}

#[test]
fn mutual_requires_both_directions_in_the_same_window() {
    let t = InteractionTracker::new();
    t.record_delivery(InteractionRoute::Proximity, h("alice"), h("bob"));
    t.record_delivery(InteractionRoute::Proximity, h("bob"), h("alice"));

    assert_eq!(t.counts(InteractionRoute::Proximity).mutual, 2);
}

#[test]
fn one_direction_per_window_is_never_mutual() {
    let t = InteractionTracker::new();
    t.record_delivery(InteractionRoute::Proximity, h("alice"), h("bob"));
    let first = t.close_window();
    t.record_delivery(InteractionRoute::Proximity, h("bob"), h("alice"));
    let second = t.close_window();

    assert_eq!(first[0].1.mutual, 0);
    assert_eq!(second[0].1.mutual, 0);
}

// Distinct-player counts do not sum across routes. alice talks on both, so the
// route rows total 4 while the real distinct figure is 3. `any` is stored rather
// than derived precisely so nothing downstream adds the two and gets it wrong.
#[test]
fn any_deduplicates_a_player_active_on_both_routes() {
    let t = InteractionTracker::new();
    t.record_delivery(InteractionRoute::Proximity, h("alice"), h("bob"));
    t.record_delivery(InteractionRoute::Channel, h("alice"), h("carol"));

    assert_eq!(t.counts(InteractionRoute::Proximity).reached, 2);
    assert_eq!(t.counts(InteractionRoute::Channel).reached, 2);
    assert_eq!(t.counts(InteractionRoute::Any).reached, 3);
}

#[test]
fn close_window_reports_every_route() {
    let t = InteractionTracker::new();
    t.record_delivery(InteractionRoute::Channel, h("alice"), h("bob"));
    let closed = t.close_window();

    let labels: Vec<&str> = closed.iter().map(|(r, _)| r.label()).collect();
    assert_eq!(labels, vec!["proximity", "channel", "any"]);
    assert_eq!(closed[0].1.reached, 0);
    assert_eq!(closed[1].1.reached, 2);
    assert_eq!(closed[2].1.reached, 2);
}
