use bvc_client_lib::testkit::bridge::OutMsg;
use bvc_client_lib::testkit::connect::Connector;
use common::structs::channel::ChannelEvents;

use crate::stdout_bridge::StdoutBridge;

// Drives an explicit channel-membership operation on the Tauri runtime and
// emits a completion frame. Spawned onto the async runtime so the synchronous
// stdin reader thread is never blocked on network I/O.
pub struct ChannelDriver;

impl ChannelDriver {
    pub fn run(handle: &tauri::AppHandle, channel_id: String, event: ChannelEvents, op: &'static str) {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            match Connector::channel_event(&handle, channel_id, event).await {
                Ok(()) => StdoutBridge::emit(&OutMsg::ChannelOpDone { op: op.to_string() }),
                Err(e) => StdoutBridge::emit(&OutMsg::Log {
                    line: format!("channel {op} failed: {e}"),
                }),
            }
        });
    }
}
