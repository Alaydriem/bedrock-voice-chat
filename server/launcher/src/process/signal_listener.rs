use std::sync::mpsc::Sender;
use std::thread;

use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use super::SupervisorEvent;

pub struct SignalListener;

impl SignalListener {
    pub fn spawn(events: Sender<SupervisorEvent>) -> anyhow::Result<()> {
        let mut signals = Signals::new([SIGTERM, SIGINT, SIGHUP])?;
        thread::spawn(move || {
            for signal in signals.forever() {
                if events.send(SupervisorEvent::Signal(signal)).is_err() {
                    return;
                }
            }
        });
        Ok(())
    }
}
