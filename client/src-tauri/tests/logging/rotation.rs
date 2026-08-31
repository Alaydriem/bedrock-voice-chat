use bvc_client_lib::logging::{LOG_ARCHIVES_KEPT, LOG_MAX_FILE_SIZE};

// curia's DEFAULT_MAX_FILE_SIZE is 40_000 bytes. A client that takes the default throws
// its log away several times a minute at debug level, so a user who reports a fault has
// already lost the minutes that produced it.
//
// A value assertion, which is normally worth nothing. It earns its place because the value
// is the contract: the defect is silently inheriting an upstream default, and this is what
// notices if the override is deleted.
#[test]
fn the_log_file_budget_is_large_enough_to_hold_a_session() {
    assert_eq!(LOG_MAX_FILE_SIZE, 25 * 1024 * 1024);
}

// curia's DEFAULT_ROTATION_STRATEGY is KeepOne, which "discards the file it rotates and
// creates no archive". KeepSome(0) behaves identically, so the count has to be positive
// for any history to survive a rotation.
#[test]
fn rotation_keeps_archives_rather_than_discarding_them() {
    assert!(
        LOG_ARCHIVES_KEPT > 0,
        "KeepSome(0) behaves exactly like KeepOne and discards every rotation"
    );
}
