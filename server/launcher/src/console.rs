/// The launcher's own messages. stdout is the panel console, so these share it with BDS and BVC.
pub struct Console;

impl Console {
    const PREFIX: &'static str = "[BVC Launcher]";

    pub fn info(message: &str) {
        println!("{} {}", Self::PREFIX, message);
    }

    pub fn warn(message: &str) {
        println!("{} WARN {}", Self::PREFIX, message);
    }

    pub fn error(message: &str) {
        println!("{} ERROR {}", Self::PREFIX, message);
    }
}
