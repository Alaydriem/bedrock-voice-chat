use std::io::Write as _;

use bvc_client_lib::testkit::bridge::{Frame, OutMsg};

// Stdout is shared between the capture-drain thread and any ad-hoc logging, so
// every framed write goes through one lock to avoid interleaving frames.
pub struct StdoutBridge;

impl StdoutBridge {
    pub fn emit(msg: &OutMsg) {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        if Frame::write(&mut lock, msg).is_err() {
            return;
        }
        let _ = lock.flush();
    }
}
