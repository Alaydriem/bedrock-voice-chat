use std::ffi::CStr;

use crate::harness::ffi::ServerLibrary;

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
