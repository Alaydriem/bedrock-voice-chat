use common::curia::{ConsoleSink, FileSink, Level, LogEvent, Sink};

pub enum LogSinkType {
    Console(ConsoleSink),
    File(FileSink),
}

impl Sink for LogSinkType {
    fn level(&self) -> Level {
        match self {
            Self::Console(s) => s.level(),
            Self::File(s) => s.level(),
        }
    }

    fn emit(&self, event: &LogEvent) {
        match self {
            Self::Console(s) => s.emit(event),
            Self::File(s) => s.emit(event),
        }
    }
}
