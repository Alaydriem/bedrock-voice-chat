use std::sync::Arc;

use tauri_plugin_curia::curia::{Level, LogEvent, Sink};
use tauri::Wry;
use tauri_plugin_curia::{ConsoleSink, FileSink, WebviewSink};

use super::SentrySink;

#[cfg(feature = "bedrock-protocol")]
use super::BedrockSink;

pub enum LogSinkType {
    Console(ConsoleSink),
    File(FileSink),
    #[allow(dead_code)]
    Webview(WebviewSink<Wry>),
    // Arc so the same sink can be managed for the exit-time drain
    Sentry(Arc<SentrySink>),
    #[cfg(feature = "bedrock-protocol")]
    Bedrock(BedrockSink),
}

impl Sink for LogSinkType {
    fn level(&self) -> Level {
        match self {
            Self::Console(s) => s.level(),
            Self::File(s) => s.level(),
            Self::Webview(s) => s.level(),
            Self::Sentry(s) => s.level(),
            #[cfg(feature = "bedrock-protocol")]
            Self::Bedrock(s) => s.level(),
        }
    }

    fn emit(&self, event: &LogEvent) {
        match self {
            Self::Console(s) => s.emit(event),
            Self::File(s) => s.emit(event),
            Self::Webview(s) => s.emit(event),
            Self::Sentry(s) => s.emit(event),
            #[cfg(feature = "bedrock-protocol")]
            Self::Bedrock(s) => s.emit(event),
        }
    }
}
