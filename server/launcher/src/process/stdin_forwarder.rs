use std::io::{self, ErrorKind, Read};
use std::sync::mpsc::Sender;
use std::thread;

use super::{BdsInput, ConsoleLine, LineSplitter, SupervisorEvent};
use crate::Console;

/// Copies the panel console to BDS line by line, except `bvc` lines, which go to the
/// supervisor and never reach BDS. At end of input the thread ends but the BDS pipe stays
/// open, because `BdsInput` still holds it.
pub struct StdinForwarder;

impl StdinForwarder {
    const BUFFER: usize = 4096;

    pub fn spawn(input: BdsInput, events: Sender<SupervisorEvent>) {
        thread::spawn(move || {
            let mut buffer = [0u8; Self::BUFFER];
            let mut splitter = LineSplitter::new();
            let mut stdin = io::stdin().lock();
            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        for line in splitter.push(&buffer[..read]) {
                            Self::route(&line, &input, &events);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            if let Some(line) = splitter.finish() {
                Self::route(&line, &input, &events);
            }
        });
    }

    fn route(line: &[u8], input: &BdsInput, events: &Sender<SupervisorEvent>) {
        match ConsoleLine::parse(line) {
            ConsoleLine::Forward => input.write(line),
            ConsoleLine::Admin(command) => {
                let _ = events.send(SupervisorEvent::Command(command));
            }
            ConsoleLine::Usage => Console::info(ConsoleLine::USAGE),
        }
    }
}
