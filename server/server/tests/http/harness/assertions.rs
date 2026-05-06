//! Tiny status-code helpers so test failures point at the test, not at this file.

#[track_caller]
pub fn assert_status(actual: u16, expected: u16) {
    assert_eq!(
        actual, expected,
        "expected HTTP {}, got HTTP {}",
        expected, actual
    );
}
