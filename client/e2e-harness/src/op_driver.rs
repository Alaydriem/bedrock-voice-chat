use std::future::Future;

use bvc_client_lib::testkit::bridge::OutMsg;

use crate::stdout_bridge::StdoutBridge;

// Runs one named operation on the Tauri runtime and reports how it ended: ChannelOpDone, or
// ChannelOpFailed with the error the production path returned, so a scenario can assert a
// refusal rather than time out on it.
pub struct OpDriver;

impl OpDriver {
    pub fn run<F, Fut>(handle: &tauri::AppHandle, op: &'static str, action: F)
    where
        F: FnOnce(tauri::AppHandle) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            match action(handle).await {
                Ok(()) => StdoutBridge::emit(&OutMsg::ChannelOpDone { op: op.to_string() }),
                Err(error) => StdoutBridge::emit(&OutMsg::ChannelOpFailed {
                    op: op.to_string(),
                    error,
                }),
            }
        });
    }
}
