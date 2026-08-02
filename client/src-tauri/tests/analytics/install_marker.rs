use bvc_client_lib::InstallMarker;
use chrono::TimeZone;

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0).unwrap()
}

#[test]
fn a_missing_id_mints_one_and_marks_the_launch_as_first() {
    let marker = InstallMarker::resolve(None, None, now());

    assert!(marker.is_first_run);
    assert_eq!(marker.install_date, "2026-07-31");
    assert!(!marker.install_id.is_empty());
}

// UUIDv7 embeds its creation time, so the install date of every id already in the
// wild is recoverable — there is no null cohort to backfill.
#[test]
fn an_existing_v7_id_recovers_its_install_date() {
    // 0x018256b3cc00 ms since the Unix epoch is 2022-08-01T00:00:00Z.
    let stored = "018256b3-cc00-7000-8000-000000000000";
    let marker = InstallMarker::resolve(Some(stored), None, now());

    assert!(!marker.is_first_run);
    assert_eq!(marker.install_id, stored);
    assert_eq!(marker.install_date, "2022-08-01");
}

// A v1 id also carries a timestamp, but from the Gregorian epoch. It must be
// rejected rather than decoded, or a legacy install reads as a 1998 cohort.
#[test]
fn a_non_v7_id_falls_back_to_the_persisted_date() {
    let stored = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
    let marker = InstallMarker::resolve(Some(stored), Some("2025-01-15"), now());

    assert!(!marker.is_first_run);
    assert_eq!(marker.install_date, "2025-01-15");
}

// A v1 id with nothing persisted predates the marker; today is the honest floor,
// and it is written once so it stays stable from then on.
#[test]
fn a_non_v7_id_with_no_persisted_date_stamps_today() {
    let stored = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
    let marker = InstallMarker::resolve(Some(stored), None, now());

    assert!(!marker.is_first_run);
    assert_eq!(marker.install_date, "2026-07-31");
}

// The nil UUID is the sentinel the existing resolution already rejects.
#[test]
fn a_nil_or_blank_id_is_treated_as_absent() {
    let nil = InstallMarker::resolve(Some("00000000-0000-0000-0000-000000000000"), None, now());
    let blank = InstallMarker::resolve(Some(""), None, now());

    assert!(nil.is_first_run);
    assert!(blank.is_first_run);
}
