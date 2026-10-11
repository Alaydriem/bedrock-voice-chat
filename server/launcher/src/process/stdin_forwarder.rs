use std::io::{self, ErrorKind, Read};
use std::thread;

use super::BdsInput;

/// Copies the panel console to BDS byte for byte. At end of input the thread ends but the BDS
/// pipe stays open, because `BdsInput` still holds it.
pub struct StdinForwarder;

impl StdinForwarder {
    const BUFFER: usize = 4096;

    pub fn spawn(input: BdsInput) {
        thread::spawn(move || {
            let mut buffer = [0u8; Self::BUFFER];
            let mut stdin = io::stdin().lock();
            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) => return,
                    Ok(read) => input.write(&buffer[..read]),
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(_) => return,
                }
            }
        });
    }
}
