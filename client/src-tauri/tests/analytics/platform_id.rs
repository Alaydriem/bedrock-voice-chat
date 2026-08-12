use bvc_client_lib::{InstallMarker, PlatformId};
use chrono::TimeZone;

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 8, 11, 12, 0, 0).unwrap()
}

// A refreshed id must not carry a timestamp. `InstallMarker` prefers the date
// embedded in a v7 id over the persisted one, so minting the replacement as v7
// would move the install into the cohort of the day it was refreshed and keep
// doing so on every refresh after that.
#[test]
fn a_refreshed_id_keeps_the_recorded_install_date() {
    let refreshed = PlatformId::generate();

    let marker = InstallMarker::resolve(Some(&refreshed), Some("2025-01-15"), now());

    assert!(!marker.is_first_run);
    assert_eq!(marker.install_id, refreshed);
    assert_eq!(marker.install_date, "2025-01-15");
}
