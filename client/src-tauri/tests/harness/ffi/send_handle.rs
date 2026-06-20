use super::RuntimeHandlePtr;

/// Wrapper making the raw handle pointer `Send` so it can cross into the
/// dedicated server thread. The library's FFI contract permits `bvc_server_stop`
/// from any thread and `bvc_server_start` from one dedicated thread, so moving
/// the pointer is sound for this single-owner usage.
#[derive(Clone, Copy)]
pub struct SendHandle(pub RuntimeHandlePtr);

unsafe impl Send for SendHandle {}
