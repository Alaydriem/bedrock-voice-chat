//! Tiny status-code helper so test failures point at the test, not at this file.

pub struct HttpAssert;

impl HttpAssert {
    #[track_caller]
    pub fn status(actual: u16, expected: u16) {
        assert_eq!(
            actual, expected,
            "expected HTTP {}, got HTTP {}",
            expected, actual
        );
    }
}
