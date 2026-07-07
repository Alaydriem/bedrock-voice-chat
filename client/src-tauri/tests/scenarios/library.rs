use std::ffi::CStr;
use std::sync::Arc;

use crate::harness::ffi::ServerLibrary;
use crate::harness::server::EmbeddedServer;

/// Load the server cdylib and call `bvc_version`, asserting a non-empty version
/// string comes back. Proves the cdylib loads and its symbols resolve before
/// any server is booted.
#[test]
fn cdylib_loads_and_reports_version() {
    let path = ServerLibrary::default_lib_path();
    let lib = ServerLibrary::load(&path).expect("load server cdylib");

    let ptr = unsafe { (lib.version)() };
    assert!(!ptr.is_null(), "bvc_version returned null");

    let version = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .expect("version is valid utf-8");
    assert!(!version.is_empty(), "bvc_version returned an empty string");
}

/// Every `load_library` call must hand back the same process-lifetime handle.
/// If this regresses to a load-per-call, the last drop during test teardown
/// unmaps the DLL while timed-out `bvc_server_destroy` threads may still be
/// executing its code — the intermittent 0xc0000005 teardown crash.
#[test]
fn load_library_returns_process_lifetime_handle() {
    let first = EmbeddedServer::load_library();
    let second = EmbeddedServer::load_library();
    assert!(
        Arc::ptr_eq(&first, &second),
        "load_library must cache one library handle for the whole process"
    );
}
