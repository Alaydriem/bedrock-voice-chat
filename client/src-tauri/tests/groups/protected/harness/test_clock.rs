use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bvc_client_lib::groups::protected::Clock;

/// A clock every member shares, advanced by the test.
#[derive(Clone)]
pub struct TestClock {
    now: Arc<Mutex<Instant>>,
}

impl TestClock {
    pub fn new() -> Self {
        Self {
            now: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn advance(&self, by: Duration) {
        let mut now = self.now.lock().unwrap();
        *now += by;
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        *self.now.lock().unwrap()
    }
}
