use bvc_server_lib::services::metrics_service::interaction::{InteractionRoute, RouteRejection};
use bvc_server_lib::services::metrics_service::route::RouteFrameReport;

// A tally is stored by `RouteRejection::index` and read back through `ALL`. Those agree only
// while `ALL` lists the variants in declaration order; if they drift, a rejection is reported
// under another reason's label.
#[test]
fn every_reason_is_reported_back_as_itself() {
    for reason in RouteRejection::ALL {
        let mut report = RouteFrameReport::new(None);
        report.reject(reason);
        report.reject(reason);

        let tallied: Vec<(RouteRejection, u32)> = report.rejections().collect();
        assert_eq!(tallied, vec![(reason, 2)], "{reason:?} was reported as something else");
    }
}

// A frame whose sender is not a live player has no interactions to measure, so collecting its
// recipients would be an allocation nothing reads.
#[test]
fn a_report_without_a_sender_collects_no_deliveries() {
    let mut report = RouteFrameReport::new(None);
    report.deliver(InteractionRoute::Proximity, 7);

    assert!(report.deliveries().is_empty());
}

#[test]
fn a_report_with_a_sender_collects_each_delivery() {
    let mut report = RouteFrameReport::new(Some(1));
    report.deliver(InteractionRoute::Proximity, 7);
    report.deliver(InteractionRoute::Channel, 8);

    assert_eq!(
        report.deliveries(),
        &[(InteractionRoute::Proximity, 7), (InteractionRoute::Channel, 8)]
    );
}
